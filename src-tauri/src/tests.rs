#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::io::Cursor;
    use crate::db::DuckDbManager;
    use crate::engine::json_stream::JsonStreamParser;
    use crate::engine::xml_stream::XmlStreamParser;

    #[test]
    fn test_xml_stream_parsing() {
        let xml_data = r#"
            <source>
                <job>
                    <id>101</id>
                    <title>Rust Developer</title>
                    <company>FastCorp</company>
                    <location>
                        <city>Berlin</city>
                        <country>DE</country>
                    </location>
                </job>
                <job>
                    <id>102</id>
                    <title>Frontend Engineer</title>
                    <company>FastCorp</company>
                    <location>
                        <city>London</city>
                        <country></country>
                    </location>
                </job>
            </source>
        "#;

        let cursor = Cursor::new(xml_data);
        let mut parser = XmlStreamParser::new(cursor, None);

        let rec1 = parser.next_record().unwrap().expect("Record 1 should exist");
        assert_eq!(rec1.fields.get("id").map(|s| s.as_str()), Some("101"));
        assert_eq!(rec1.fields.get("title").map(|s| s.as_str()), Some("Rust Developer"));
        assert_eq!(rec1.fields.get("location.city").map(|s| s.as_str()), Some("Berlin"));
        assert_eq!(rec1.fields.get("location.country").map(|s| s.as_str()), Some("DE"));

        let rec2 = parser.next_record().unwrap().expect("Record 2 should exist");
        assert_eq!(rec2.fields.get("id").map(|s| s.as_str()), Some("102"));
        assert_eq!(rec2.fields.get("title").map(|s| s.as_str()), Some("Frontend Engineer"));

        let rec3 = parser.next_record().unwrap();
        assert!(rec3.is_none());
    }

    #[test]
    fn test_json_stream_parsing() {
        let json_data = "{\"id\":\"J1\",\"title\":\"Data Engineer\",\"company\":{\"name\":\"Alpha\",\"city\":\"Paris\"}}\n{\"id\":\"J2\",\"title\":\"AI Engineer\",\"company\":{\"name\":\"Beta\",\"city\":\"NYC\"}}";
        let cursor = Cursor::new(json_data);
        let mut parser = JsonStreamParser::new(cursor);

        let rec1 = parser.next_record().unwrap().expect("Record 1 should exist");
        assert_eq!(rec1.fields.get("id").map(|s| s.as_str()), Some("J1"));
        assert_eq!(rec1.fields.get("title").map(|s| s.as_str()), Some("Data Engineer"));
        assert_eq!(rec1.fields.get("company.city").map(|s| s.as_str()), Some("Paris"));

        let rec2 = parser.next_record().unwrap().expect("Record 2 should exist");
        assert_eq!(rec2.fields.get("id").map(|s| s.as_str()), Some("J2"));

        let rec3 = parser.next_record().unwrap();
        assert!(rec3.is_none());
    }

    #[test]
    fn test_duckdb_analytics() {
        let db = DuckDbManager::new().expect("DuckDB should init");
        let initial_cols = vec!["id".to_string(), "title".to_string(), "city".to_string()];
        db.init_table(&initial_cols).unwrap();

        let mut row1 = HashMap::new();
        row1.insert("id".to_string(), "1".to_string());
        row1.insert("title".to_string(), "Rust Specialist".to_string());
        row1.insert("city".to_string(), "San Francisco".to_string());

        let mut row2 = HashMap::new();
        row2.insert("id".to_string(), "2".to_string());
        row2.insert("title".to_string(), "Rust Specialist".to_string());
        row2.insert("city".to_string(), "".to_string()); // empty city

        let mut row3 = HashMap::new();
        row3.insert("id".to_string(), "2".to_string()); // duplicate ID 2!
        row3.insert("title".to_string(), "Go Engineer".to_string());
        row3.insert("city".to_string(), "London".to_string());

        db.insert_batch(&[row1, row2, row3]).unwrap();

        let overview = db.get_overview(1.0).unwrap();
        assert_eq!(overview.total_records, 3);

        let matrix = db.get_completeness_matrix().unwrap();
        let city_metric = matrix.iter().find(|m| m.original_name == "city").unwrap();
        assert_eq!(city_metric.total_records, 3);
        assert_eq!(city_metric.empty_count, 1);
        assert_eq!(city_metric.valid_count, 2);

        // Check duplicate detection
        let duplicates = db.get_duplicates("id", 10).unwrap();
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0].key, "2");
        assert_eq!(duplicates[0].count, 2);

        // Check value frequency
        let freq = db.get_column_distribution("title", 10).unwrap();
        assert_eq!(freq[0].value, "Rust Specialist");
        assert_eq!(freq[0].count, 2);
    }
}
