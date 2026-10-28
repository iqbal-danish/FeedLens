use super::DuckDbManager;

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    LParen,
    RParen,
    And,
    Or,
    Not,
    Term(String),
}

#[derive(Debug, PartialEq, Clone)]
pub enum AstNode {
    And(Vec<AstNode>),
    Or(Vec<AstNode>),
    Not(Box<AstNode>),
    Predicate(String),
}

pub struct QueryParser;

impl QueryParser {
    /// Tokenize the search input into tokens, handling quotes and injecting implicit ANDs
    pub fn tokenize(input: &str) -> Vec<Token> {
        let mut raw_tokens = Vec::new();
        let chars: Vec<char> = input.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            let ch = chars[i];

            if ch.is_whitespace() {
                i += 1;
                continue;
            }

            if ch == '(' {
                raw_tokens.push(Token::LParen);
                i += 1;
                continue;
            }

            if ch == ')' {
                raw_tokens.push(Token::RParen);
                i += 1;
                continue;
            }

            if ch == '"' {
                // Quoted term: read until closing quote
                let mut s = String::new();
                s.push('"');
                i += 1;
                while i < len && chars[i] != '"' {
                    s.push(chars[i]);
                    i += 1;
                }
                if i < len && chars[i] == '"' {
                    s.push('"');
                    i += 1;
                }
                raw_tokens.push(Token::Term(s));
                continue;
            }

            // Word or symbol sequence
            let start = i;
            let mut in_quotes = false;
            while i < len {
                let c = chars[i];
                if c == '"' {
                    in_quotes = !in_quotes;
                } else if !in_quotes && (c.is_whitespace() || c == '(' || c == ')') {
                    break;
                }
                i += 1;
            }

            let word: String = chars[start..i].iter().collect();
            let upper = word.to_uppercase();

            if upper == "AND" || upper == "&&" {
                raw_tokens.push(Token::And);
            } else if upper == "OR" || upper == "||" {
                raw_tokens.push(Token::Or);
            } else if upper == "NOT" {
                raw_tokens.push(Token::Not);
            } else {
                raw_tokens.push(Token::Term(word));
            }
        }

        // Insert implicit ANDs between adjacent terms, parens, and NOTs
        let mut tokens = Vec::new();
        for (idx, tok) in raw_tokens.iter().enumerate() {
            if idx > 0 {
                let prev = &raw_tokens[idx - 1];
                let prev_can_end = matches!(prev, Token::Term(_) | Token::RParen);
                let curr_can_start = matches!(tok, Token::Term(_) | Token::LParen | Token::Not);

                if prev_can_end && curr_can_start {
                    tokens.push(Token::And);
                }
            }
            tokens.push(tok.clone());
        }

        tokens
    }

    /// Parse tokens into an AST using recursive descent
    pub fn parse(tokens: &[Token]) -> Option<AstNode> {
        let mut pos = 0;
        let node = Self::parse_or(tokens, &mut pos);
        if pos < tokens.len() && node.is_some() {
            // If trailing tokens remain, wrap in an And
            let mut nodes = vec![node.unwrap()];
            while pos < tokens.len() {
                if let Some(n) = Self::parse_or(tokens, &mut pos) {
                    nodes.push(n);
                } else {
                    pos += 1;
                }
            }
            Some(AstNode::And(nodes))
        } else {
            node
        }
    }

    fn parse_or(tokens: &[Token], pos: &mut usize) -> Option<AstNode> {
        let mut left = Self::parse_and(tokens, pos)?;

        while *pos < tokens.len() {
            if let Token::Or = tokens[*pos] {
                *pos += 1;
                if let Some(right) = Self::parse_and(tokens, pos) {
                    left = match left {
                        AstNode::Or(mut list) => {
                            list.push(right);
                            AstNode::Or(list)
                        }
                        _ => AstNode::Or(vec![left, right]),
                    };
                }
            } else {
                break;
            }
        }

        Some(left)
    }

    fn parse_and(tokens: &[Token], pos: &mut usize) -> Option<AstNode> {
        let mut left = Self::parse_unary(tokens, pos)?;

        while *pos < tokens.len() {
            if let Token::And = tokens[*pos] {
                *pos += 1;
                if let Some(right) = Self::parse_unary(tokens, pos) {
                    left = match left {
                        AstNode::And(mut list) => {
                            list.push(right);
                            AstNode::And(list)
                        }
                        _ => AstNode::And(vec![left, right]),
                    };
                }
            } else {
                break;
            }
        }

        Some(left)
    }

    fn parse_unary(tokens: &[Token], pos: &mut usize) -> Option<AstNode> {
        if *pos < tokens.len() {
            if let Token::Not = tokens[*pos] {
                *pos += 1;
                let child = Self::parse_unary(tokens, pos)?;
                return Some(AstNode::Not(Box::new(child)));
            }
        }
        Self::parse_primary(tokens, pos)
    }

    fn parse_primary(tokens: &[Token], pos: &mut usize) -> Option<AstNode> {
        if *pos >= tokens.len() {
            return None;
        }

        match &tokens[*pos] {
            Token::LParen => {
                *pos += 1;
                let expr = Self::parse_or(tokens, pos);
                if *pos < tokens.len() && tokens[*pos] == Token::RParen {
                    *pos += 1;
                }
                expr
            }
            Token::Term(s) => {
                *pos += 1;
                Some(AstNode::Predicate(s.clone()))
            }
            _ => {
                *pos += 1;
                None
            }
        }
    }

    /// Helper to parse numeric strings like "80000", "$80,000", "80k", "1.5m"
    pub fn parse_numeric_value(s: &str) -> Option<f64> {
        let clean = s.trim().replace('$', "").replace(',', "").replace(' ', "");
        if clean.is_empty() {
            return None;
        }

        let low = clean.to_lowercase();
        if let Some(val_str) = low.strip_suffix('k') {
            val_str.parse::<f64>().ok().map(|v| v * 1_000.0)
        } else if let Some(val_str) = low.strip_suffix('m') {
            val_str.parse::<f64>().ok().map(|v| v * 1_000_000.0)
        } else {
            clean.parse::<f64>().ok()
        }
    }

    /// Check if string looks like an ISO date (YYYY-MM-DD)
    pub fn is_iso_date(s: &str) -> bool {
        let t = s.trim();
        if t.len() == 10 {
            let bytes = t.as_bytes();
            bytes[4] == b'-' && bytes[7] == b'-' &&
            bytes[0..4].iter().all(|b| b.is_ascii_digit()) &&
            bytes[5..7].iter().all(|b| b.is_ascii_digit()) &&
            bytes[8..10].iter().all(|b| b.is_ascii_digit())
        } else {
            false
        }
    }

    /// Convert a single predicate term to SQL condition
    pub fn predicate_to_sql(term: &str, cols: &[String]) -> String {
        let trimmed = term.trim();
        if trimmed.is_empty() {
            return String::new();
        }

        // Negation prefix: -term or !term
        if let Some(rest) = trimmed.strip_prefix('-').or_else(|| trimmed.strip_prefix('!')) {
            if !rest.is_empty() {
                let inner = Self::predicate_to_sql(rest, cols);
                if !inner.is_empty() {
                    return format!("NOT ({})", inner);
                }
            }
        }

        // Missing / Populated checks
        if let Some(target) = trimmed.strip_prefix("missing:").or_else(|| trimmed.strip_prefix("is:empty:")) {
            if let Some(c) = DuckDbManager::resolve_column_name(target.trim(), cols) {
                return format!("(\"{}\" IS NULL OR TRIM(\"{}\") = '')", c, c);
            }
        } else if let Some(target) = trimmed.strip_prefix("populated:").or_else(|| trimmed.strip_prefix("is:valid:")) {
            if let Some(c) = DuckDbManager::resolve_column_name(target.trim(), cols) {
                return format!("(\"{}\" IS NOT NULL AND TRIM(\"{}\") != '')", c, c);
            }
        }

        // Split by operators: !=, >=, <=, >, <, =, :
        let op_candidates = [
            ("!=", "!="),
            ("<>", "!="),
            (":!=", "!="),
            (":>=", ">="),
            (":<=", "<="),
            (":>", ">"),
            (":<", "<"),
            (">=", ">="),
            ("<=", "<="),
            (">", ">"),
            ("<", "<"),
            ("=", "="),
            (":", "="),
        ];

        for (pattern, op) in op_candidates {
            if let Some((raw_col, raw_val)) = trimmed.split_once(pattern) {
                let col_name = raw_col.trim();
                let val = raw_val.trim().trim_matches('"');
                if let Some(c) = DuckDbManager::resolve_column_name(col_name, cols) {
                    let escaped_val = val.replace('\'', "''");

                    // 1. Range syntax: min..max (e.g. salary:50000..100000 or salary:50k..100k or date:2026-01-01..2026-06-30)
                    if op == "=" && val.contains("..") {
                        if let Some((min_str, max_str)) = val.split_once("..") {
                            let min_num = Self::parse_numeric_value(min_str);
                            let max_num = Self::parse_numeric_value(max_str);

                            if let (Some(min), Some(max)) = (min_num, max_num) {
                                return format!(
                                    "TRY_CAST(REGEXP_REPLACE(CAST(\"{}\" AS VARCHAR), '[^0-9.]', '', 'g') AS DOUBLE) BETWEEN {} AND {}",
                                    c, min, max
                                );
                            } else if Self::is_iso_date(min_str) && Self::is_iso_date(max_str) {
                                return format!(
                                    "TRY_CAST(\"{}\" AS DATE) BETWEEN DATE '{}' AND DATE '{}'",
                                    c, min_str.trim(), max_str.trim()
                                );
                            }
                        }
                    }

                    // 2. Comparison operators: >, >=, <, <=
                    if matches!(op, ">" | ">=" | "<" | "<=") {
                        if let Some(num) = Self::parse_numeric_value(val) {
                            return format!(
                                "TRY_CAST(REGEXP_REPLACE(CAST(\"{}\" AS VARCHAR), '[^0-9.]', '', 'g') AS DOUBLE) {} {}",
                                c, op, num
                            );
                        } else if Self::is_iso_date(val) {
                            return format!(
                                "TRY_CAST(\"{}\" AS DATE) {} DATE '{}'",
                                c, op, val
                            );
                        } else {
                            return format!("CAST(\"{}\" AS VARCHAR) {} '{}'", c, op, escaped_val);
                        }
                    }

                    // 3. Inequality: !=
                    if op == "!=" {
                        if val.eq_ignore_ascii_case("true") || val == "1" || val.eq_ignore_ascii_case("yes") {
                            return format!(
                                "(\"{}\" IS NULL OR LOWER(TRIM(CAST(\"{}\" AS VARCHAR))) NOT IN ('true', '1', 'yes'))",
                                c, c
                            );
                        } else {
                            return format!(
                                "(\"{}\" IS NULL OR CAST(\"{}\" AS VARCHAR) NOT ILIKE '%{}%')",
                                c, c, escaped_val
                            );
                        }
                    }

                    // 4. Boolean shortcut equality
                    if val.eq_ignore_ascii_case("true") || val == "1" || val.eq_ignore_ascii_case("yes") {
                        return format!(
                            "(LOWER(TRIM(CAST(\"{}\" AS VARCHAR))) IN ('true', '1', 'yes') OR CAST(\"{}\" AS VARCHAR) ILIKE '%true%')",
                            c, c
                        );
                    } else if val.eq_ignore_ascii_case("false") || val == "0" || val.eq_ignore_ascii_case("no") {
                        return format!(
                            "(\"{}\" IS NULL OR LOWER(TRIM(CAST(\"{}\" AS VARCHAR))) IN ('false', '0', 'no', '') OR CAST(\"{}\" AS VARCHAR) ILIKE '%false%')",
                            c, c, c
                        );
                    }

                    // 5. Exact match if value was enclosed in quotes in original input
                    if raw_val.trim().starts_with('"') && raw_val.trim().ends_with('"') {
                        return format!("LOWER(TRIM(CAST(\"{}\" AS VARCHAR))) = LOWER('{}')", c, escaped_val);
                    }

                    // Default string contains
                    return format!("CAST(\"{}\" AS VARCHAR) ILIKE '%{}%'", c, escaped_val);
                }
            }
        }

        // Global free text search across all columns
        let escaped = trimmed.trim_matches('"').replace('\'', "''");
        let conditions: Vec<String> = cols
            .iter()
            .map(|c| format!("CAST(\"{}\" AS VARCHAR) ILIKE '%{}%'", c, escaped))
            .collect();
        if !conditions.is_empty() {
            format!("({})", conditions.join(" OR "))
        } else {
            String::new()
        }
    }

    /// Recursively build SQL expression from AST
    pub fn ast_to_sql(node: &AstNode, cols: &[String]) -> String {
        match node {
            AstNode::And(children) => {
                let parts: Vec<String> = children
                    .iter()
                    .map(|c| Self::ast_to_sql(c, cols))
                    .filter(|s| !s.is_empty())
                    .collect();
                if parts.is_empty() {
                    String::new()
                } else if parts.len() == 1 {
                    parts[0].clone()
                } else {
                    format!("({})", parts.join(" AND "))
                }
            }
            AstNode::Or(children) => {
                let parts: Vec<String> = children
                    .iter()
                    .map(|c| Self::ast_to_sql(c, cols))
                    .filter(|s| !s.is_empty())
                    .collect();
                if parts.is_empty() {
                    String::new()
                } else if parts.len() == 1 {
                    parts[0].clone()
                } else {
                    format!("({})", parts.join(" OR "))
                }
            }
            AstNode::Not(child) => {
                let inner = Self::ast_to_sql(child, cols);
                if inner.is_empty() {
                    String::new()
                } else {
                    format!("NOT ({})", inner)
                }
            }
            AstNode::Predicate(s) => Self::predicate_to_sql(s, cols),
        }
    }

    /// Master entry point to build SQL WHERE clause
    pub fn build_where_clause(filter_text: &Option<String>, cols: &[String]) -> String {
        if let Some(ref text) = filter_text {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return String::new();
            }

            let tokens = Self::tokenize(trimmed);
            if tokens.is_empty() {
                return String::new();
            }

            if let Some(ast) = Self::parse(&tokens) {
                let sql = Self::ast_to_sql(&ast, cols);
                if !sql.is_empty() {
                    return format!("WHERE {}", sql);
                }
            }
        }
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenization_basic_and_implicit() {
        let toks = QueryParser::tokenize("remote:true city:London");
        assert_eq!(
            toks,
            vec![
                Token::Term("remote:true".to_string()),
                Token::And,
                Token::Term("city:London".to_string())
            ]
        );
    }

    #[test]
    fn test_tokenization_explicit_or_and_parens() {
        let toks = QueryParser::tokenize("(city:London OR city:Paris) AND remote:true");
        assert_eq!(
            toks,
            vec![
                Token::LParen,
                Token::Term("city:London".to_string()),
                Token::Or,
                Token::Term("city:Paris".to_string()),
                Token::RParen,
                Token::And,
                Token::Term("remote:true".to_string())
            ]
        );
    }

    #[test]
    fn test_numeric_parsing() {
        assert_eq!(QueryParser::parse_numeric_value("80000"), Some(80000.0));
        assert_eq!(QueryParser::parse_numeric_value("$80,000"), Some(80000.0));
        assert_eq!(QueryParser::parse_numeric_value("80k"), Some(80000.0));
        assert_eq!(QueryParser::parse_numeric_value("120.5k"), Some(120500.0));
        assert_eq!(QueryParser::parse_numeric_value("1.5m"), Some(1500000.0));
    }

    #[test]
    fn test_ast_building_precedence() {
        let tokens = QueryParser::tokenize("a OR b AND c");
        let ast = QueryParser::parse(&tokens).unwrap();
        // Should parse as: a OR (b AND c) because AND has higher precedence
        match ast {
            AstNode::Or(children) => {
                assert_eq!(children.len(), 2);
                assert_eq!(children[0], AstNode::Predicate("a".to_string()));
                match &children[1] {
                    AstNode::And(and_children) => {
                        assert_eq!(and_children.len(), 2);
                    }
                    _ => panic!("Expected AND inside OR"),
                }
            }
            _ => panic!("Expected OR at root"),
        }
    }

    #[test]
    fn test_sql_generation_advanced() {
        let cols = vec![
            "source_name".to_string(),
            "city".to_string(),
            "remote".to_string(),
            "salary".to_string(),
        ];

        // 1. Compound OR and implicit AND
        let sql = QueryParser::build_where_clause(
            &Some("(city:Austin OR city:Dallas) remote:true".to_string()),
            &cols,
        );
        assert!(sql.contains("WHERE ((CAST(\"city\" AS VARCHAR) ILIKE '%Austin%' OR CAST(\"city\" AS VARCHAR) ILIKE '%Dallas%') AND"));
        assert!(sql.contains("'true'"));

        // 2. Numeric range salary:50k..100k
        let sql2 = QueryParser::build_where_clause(
            &Some("salary:50k..100k".to_string()),
            &cols,
        );
        assert!(sql2.contains("BETWEEN 50000 AND 100000"));

        // 3. Numeric comparison salary:>80k
        let sql3 = QueryParser::build_where_clause(
            &Some("salary:>80k".to_string()),
            &cols,
        );
        assert!(sql3.contains("> 80000"));

        // 4. NOT keyword and prefix minus
        let sql4 = QueryParser::build_where_clause(
            &Some("NOT city:Chicago".to_string()),
            &cols,
        );
        assert!(sql4.contains("NOT (CAST(\"city\" AS VARCHAR) ILIKE '%Chicago%')"));
    }
}
