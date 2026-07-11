// Problem C integration test: schema-as-code
// Tests schema parsing, DDL generation, db_insert into schema-created table, query back.
// This is a TW-only test (schema requires SQLite connection, not available in VM/JIT).

#[cfg(test)]
mod tests {
    use metalogos::run_program;

    #[test]
    fn schema_creates_table_and_inserts() {
        let source = r#"
db { url: "sqlite::memory:" }

schema test_dept {
  table analysis {
    id: Int primary_key auto_increment
    topic: String
    status: String default("drafted")
  }
}

pattern InsertAndQuery(topic: String) -> String {
  let id = db_insert("analysis", { topic: topic, status: "drafted" })
  let rows = query("SELECT topic FROM analysis WHERE id = " + to_string(id), [])
  let first = get(rows, 0)
  return first.topic
}

flow Main {
  input: String = "test_topic" -> InsertAndQuery -> output
}
"#;
        let result = run_program(source.to_string()).expect("execution failed");
        let output = result.unwrap_or_default();
        assert_eq!(output.trim(), "test_topic", "schema + db_insert + query round-trip failed");
    }

    #[test]
    fn schema_with_all_modifiers() {
        let source = r#"
db { url: "sqlite::memory:" }

schema full_test {
  table items {
    id: Int primary_key auto_increment
    name: String
    price: Float
    notes: Text nullable
    active: Bool
  }
}

pattern InsertAndRead(name: String) -> String {
  db_insert("items", { name: name, price: 9.99, active: true })
  let rows = query("SELECT name, price FROM items", [])
  let first = get(rows, 0)
  return first.name + ":" + to_string(first.price)
}

flow Main {
  input: String = "widget" -> InsertAndRead -> output
}
"#;
        let result = run_program(source.to_string()).expect("execution failed");
        let output = result.unwrap_or_default();
        assert_eq!(output.trim(), "widget:9.99", "schema with all modifiers failed");
    }

    #[test]
    fn schema_additive_no_drop() {
        // Running schema twice should not fail (IF NOT EXISTS)
        let source = r#"
db { url: "sqlite::memory:" }

schema test_dept {
  table items {
    id: Int primary_key auto_increment
    name: String
  }
}

schema test_dept {
  table items {
    id: Int primary_key auto_increment
    name: String
  }
  table extra {
    id: Int primary_key auto_increment
    value: String
  }
}

pattern TestAdditive() -> String {
  db_insert("items", { name: "first" })
  db_insert("extra", { value: "bonus" })
  let r1 = query("SELECT name FROM items", [])
  let r2 = query("SELECT value FROM extra", [])
  return get(r1, 0).name + "+" + get(r2, 0).value
}

flow Main {
  -> TestAdditive -> output
}
"#;
        let result = run_program(source.to_string()).expect("execution failed");
        let output = result.unwrap_or_default();
        assert_eq!(output.trim(), "first+bonus", "additive schema migration failed");
    }
}