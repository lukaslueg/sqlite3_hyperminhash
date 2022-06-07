mod util;
use util::init_db;

fn expect_error_msg<T: std::fmt::Debug>(
    r: rusqlite::Result<T>,
    needle: &'static str,
    err_msg: &'static str,
) -> rusqlite::Result<()> {
    match r {
        Err(rusqlite::Error::SqliteFailure(_, Some(ref s))) if s.contains(needle) => Ok(()),
        other => {
            panic!("{} {:?}", err_msg, other);
        }
    }
}

#[cfg(feature = "serialize")]
pub mod serialize {
    use super::*;
    use hyperminhash::Sketch;

    macro_rules! test_wrong_type {
        ($name:ident, $func:literal) => {
            #[test]
            fn $name() -> rusqlite::Result<()> {
                let con = init_db()?;
                let r: rusqlite::Result<u8> =
                    con.query_row(&format!("SELECT {}", $func), rusqlite::params![], |row| {
                        row.get(0)
                    });
                expect_error_msg(r, "not of type BLOB", "did not complain about type:")
            }
        };
    }

    macro_rules! test_bad_data {
        ($name:ident, $func:literal) => {
            #[test]
            fn $name() -> rusqlite::Result<()> {
                let con = init_db()?;
                let r: rusqlite::Result<u8> =
                    con.query_row(&format!("SELECT {}", $func), rusqlite::params![], |row| {
                        row.get(0)
                    });
                expect_error_msg(
                    r,
                    "IO-error in hyperminhash",
                    "unpacked bad data without error",
                )
            }
        };
    }

    #[test]
    fn zero() -> rusqlite::Result<()> {
        let con = init_db()?;
        // Count is zero
        let buf: Vec<u8> =
            con.query_row("SELECT HYPERMINHASH_ZERO()", rusqlite::params![], |row| {
                row.get(0)
            })?;
        let sketch = Sketch::load(&buf[..]).unwrap();
        assert_eq!(sketch.cardinality(), 0.0);
        Ok(())
    }

    #[test]
    fn serialize() -> rusqlite::Result<()> {
        let con = init_db()?;
        con.execute("CREATE TABLE foo (id INT)", rusqlite::params![])?;

        // Count is zero
        let buf: Vec<u8> = con.query_row(
            "SELECT HYPERMINHASH_SERIALIZE(id) FROM foo",
            rusqlite::params![],
            |row| row.get(0),
        )?;
        let sketch = Sketch::load(&buf[..]).unwrap();
        assert_eq!(sketch.cardinality(), 0.0);

        // Count is not zero
        con.execute("INSERT INTO foo (id) VALUES (0)", rusqlite::params![])?;
        let buf: Vec<u8> = con.query_row(
            "SELECT HYPERMINHASH_SERIALIZE(id) FROM foo",
            rusqlite::params![],
            |row| row.get(0),
        )?;
        let r = Sketch::load(&buf[..]).unwrap().cardinality();
        assert!((1.0 - r).abs() < 0.05);

        Ok(())
    }

    #[test]
    fn deserialize() -> rusqlite::Result<()> {
        let sketch: Sketch = (0..100).collect();
        let mut buf: Vec<u8> = Vec::new();
        sketch.save(&mut buf).unwrap();

        let con = init_db()?;
        con.execute("CREATE TABLE counts (data BLOB)", rusqlite::params![])?;
        con.execute(
            "INSERT INTO counts (data) VALUES (?1)",
            rusqlite::params![&buf],
        )?;

        let r: f64 = con.query_row(
            "SELECT HYPERMINHASH_DESERIALIZE(data) FROM counts",
            rusqlite::params![],
            |row| row.get(0),
        )?;
        assert_eq!(r, sketch.cardinality());

        Ok(())
    }

    test_wrong_type!(deserialize_wrong_type, "HYPERMINHASH_DESERIALIZE('foo')");
    test_bad_data!(deserialize_bad_data, "HYPERMINHASH_DESERIALIZE(X'00')");

    #[test]
    fn add() -> rusqlite::Result<()> {
        let con = init_db()?;
        con.execute(
            "CREATE TABLE counts (id INT PRIMARY KEY, data BLOB)",
            rusqlite::params![],
        )?;
        con.execute(
            "INSERT INTO counts VALUES (0, HYPERMINHASH_ZERO())",
            rusqlite::params![],
        )?;

        con.execute("CREATE TABLE foo (id INT)", rusqlite::params![])?;
        let mut stmt = con.prepare("INSERT INTO foo (id) VALUES (?1)")?;
        for i in 0..200 {
            stmt.execute([i])?;
        }
        con.execute(
            r#"UPDATE counts
                       SET data = HYPERMINHASH_ADD(data,
                            (SELECT HYPERMINHASH_SERIALIZE(id)
                             FROM foo
                             WHERE id < 100)
                            )
                       WHERE counts.id = 0"#,
            rusqlite::params![],
        )?;
        let r: f64 = con.query_row(
            "SELECT HYPERMINHASH_DESERIALIZE(data) FROM counts WHERE id = 0",
            rusqlite::params![],
            |row| row.get(0),
        )?;
        assert!((1.0 - (r / 100.0)).abs() < 0.05);

        con.execute(
            r#"UPDATE counts
                       SET data = HYPERMINHASH_ADD(data,
                            (SELECT HYPERMINHASH_SERIALIZE(id)
                             FROM foo
                             WHERE id >= 100)
                            )
                       WHERE counts.id = 0"#,
            rusqlite::params![],
        )?;
        let r: f64 = con.query_row(
            "SELECT HYPERMINHASH_DESERIALIZE(data) FROM counts WHERE id = 0",
            rusqlite::params![],
            |row| row.get(0),
        )?;
        assert!((1.0 - (r / 200.0)).abs() < 0.05);

        Ok(())
    }

    test_wrong_type!(add_wrong_type, "HYPERMINHASH_ADD('foo')");
    test_bad_data!(add_bad_data, "HYPERMINHASH_ADD(X'00')");

    #[test]
    fn union() -> rusqlite::Result<()> {
        let con = init_db()?;
        con.execute("CREATE TABLE foo (id INT)", rusqlite::params![])?;
        let mut stmt = con.prepare("INSERT INTO foo (id) VALUES (?1)")?;
        for i in 0..100 {
            stmt.execute([i])?;
        }
        con.execute(
            "CREATE TABLE stats (id INT PRIMARY KEY, data BLOB)",
            rusqlite::params![],
        )?;
        con.execute(
            r#"INSERT INTO stats (id, data)
               SELECT 0, HYPERMINHASH_SERIALIZE(foo.id)
               FROM foo
               WHERE foo.id <= 50"#,
            rusqlite::params![],
        )?;
        con.execute(
            r#"INSERT INTO stats (id, data)
               SELECT 1, HYPERMINHASH_SERIALIZE(foo.id)
               FROM foo
               WHERE foo.id > 50"#,
            rusqlite::params![],
        )?;
        let r: f64 = con.query_row(
            r#"SELECT HYPERMINHASH_DESERIALIZE(
                        (SELECT HYPERMINHASH_UNION(data)
                         FROM stats
                        )
                      )"#,
            rusqlite::params![],
            |row| row.get(0),
        )?;
        assert!((1.0 - (r / 100.0)).abs() < 0.05);
        Ok(())
    }

    test_wrong_type!(union_wrong_type, "HYPERMINHASH_UNION('foo')");
    test_bad_data!(union_bad_data, "HYPERMINHASH_UNION(X'00')");

    #[test]
    fn intersection() -> rusqlite::Result<()> {
        let con = init_db()?;
        con.execute("CREATE TABLE foo (id INT)", rusqlite::params![])?;
        let mut stmt = con.prepare("INSERT INTO foo (id) VALUES (?1)")?;
        for i in 0..1000 {
            stmt.execute([i])?;
        }
        let r: f64 = con.query_row(
            "SELECT HYPERMINHASH_INTERSECTION(
                (SELECT HYPERMINHASH_SERIALIZE(id) FROM foo WHERE id < 750),
                (SELECT HYPERMINHASH_SERIALIZE(id) FROM foo WHERE id >= 250)
            )",
            rusqlite::params![],
            |row| row.get(0),
        )?;
        assert!((1.0 - (r / 500.0)).abs() < 0.05);
        Ok(())
    }

    test_wrong_type!(
        intersection_wrong_type,
        "HYPERMINHASH_INTERSECTION('foo', 'bar')"
    );
    test_bad_data!(
        intersection_bad_data,
        "HYPERMINHASH_INTERSECTION(X'00', X'00')"
    );
}

#[cfg(not(feature = "serialize"))]
pub mod serialize_stub {
    use super::*;

    macro_rules! no_such_func {
        ($name:ident, $func:literal) => {
            #[test]
            fn $name() -> rusqlite::Result<()> {
                let con = init_db()?;
                let r: rusqlite::Result<u8> =
                    con.query_row(&format!("SELECT {}", $func), rusqlite::params![], |row| {
                        row.get(0)
                    });
                expect_error_msg(r, "`serialize`-feature", "error not reported: ")
            }
        };
    }

    no_such_func!(zero_returns_error, "hyperminhash_zero()");
    no_such_func!(serialize_returns_error, "hyperminhash_serialize()");
    no_such_func!(deserialize_returns_error, "hyperminhash_deserialize(X'00')");
    no_such_func!(add_returns_error, "hyperminhash_add()");
    no_such_func!(union_returns_error, "hyperminhash_union(X'00')");
    no_such_func!(
        intersection_returns_error,
        "hyperminhash_intersection(X'00', X'00')"
    );
}
