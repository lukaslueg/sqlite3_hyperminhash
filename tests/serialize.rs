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
        assert!(r > 0.8);
        assert!(r < 1.2);

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

    #[test]
    fn deserialize_wrong_type() -> rusqlite::Result<()> {
        let con = init_db()?;
        let r: rusqlite::Result<f64> = con.query_row(
            "SELECT HYPERMINHASH_DESERIALIZE((SELECT 'foo'))",
            rusqlite::params![],
            |row| row.get(0),
        );
        expect_error_msg(r, "not of type BLOB", "did not complain about type:")
    }

    #[test]
    fn deserialize_bad_data() -> rusqlite::Result<()> {
        let con = init_db()?;
        let r: rusqlite::Result<f64> = con.query_row(
            "SELECT HYPERMINHASH_DESERIALIZE((SELECT X'00'))",
            rusqlite::params![],
            |row| row.get(0),
        );
        expect_error_msg(
            r,
            "IO-error in hyperminhash",
            "unpacked bad data without error",
        )
    }

    #[test]
    fn union() -> rusqlite::Result<()> {
        let con = init_db()?;
        con.execute(
            "CREATE TABLE foobar (foo INT, bar INT)",
            rusqlite::params![],
        )?;
        con.execute(
            "INSERT INTO foobar (foo, bar) VALUES (0, 0)",
            rusqlite::params![],
        )?;
        con.execute(
            "INSERT INTO foobar (foo, bar) VALUES (1, 1)",
            rusqlite::params![],
        )?;
        let r: f64 = con.query_row(
            r#"SELECT HYPERMINHASH_DESERIALIZE(
                        HYPERMINHASH_UNION(
                            (SELECT HYPERMINHASH_SERIALIZE(bar) FROM foobar WHERE foo = 0),
                            (SELECT HYPERMINHASH_SERIALIZE(bar) FROM foobar WHERE foo = 1)
                        )
                      )"#,
            rusqlite::params![],
            |row| row.get(0),
        )?;
        assert!(r > 1.8);
        assert!(r < 2.2);
        Ok(())
    }

    #[test]
    fn union_wrong_type() -> rusqlite::Result<()> {
        let con = init_db()?;
        let r: rusqlite::Result<Vec<u8>> = con.query_row(
            "SELECT HYPERMINHASH_UNION((SELECT 'foo'), (SELECT 'bar'))",
            rusqlite::params![],
            |row| row.get(0),
        );
        expect_error_msg(r, "not of type BLOB", "did not complain about type:")
    }

    #[test]
    fn union_bad_data() -> rusqlite::Result<()> {
        let con = init_db()?;
        let r: rusqlite::Result<Vec<u8>> = con.query_row(
            "SELECT HYPERMINHASH_UNION((SELECT X'00'), (SELECT X'00'))",
            rusqlite::params![],
            |row| row.get(0),
        );
        expect_error_msg(
            r,
            "IO-error in hyperminhash",
            "unpacked bad data without error",
        )
    }

    #[test]
    fn intersection() -> rusqlite::Result<()> {
        let con = init_db()?;
        con.execute("CREATE TABLE foo (id INT)", rusqlite::params![])?;
        let mut stmt = con.prepare("INSERT INTO foo (id) VALUES (?1)")?;
        for i in 0..1000 {
            stmt.execute(&[i])?;
        }
        let r: f64 = con.query_row(
            "SELECT HYPERMINHASH_INTERSECTION(
                (SELECT HYPERMINHASH_SERIALIZE(id) FROM foo WHERE id < 750),
                (SELECT HYPERMINHASH_SERIALIZE(id) FROM foo WHERE id >= 250)
            )",
            rusqlite::params![],
            |row| row.get(0),
        )?;
        dbg!(r);
        assert!(r > 450.0);
        assert!(r < 550.0);
        Ok(())
    }

    #[test]
    fn intersection_wrong_type() -> rusqlite::Result<()> {
        let con = init_db()?;
        let r: rusqlite::Result<f64> = con.query_row(
            "SELECT HYPERMINHASH_INTERSECTION((SELECT 'foo'), (SELECT 'bar'))",
            rusqlite::params![],
            |row| row.get(0),
        );
        expect_error_msg(r, "not of type BLOB", "did not complain about type:")
    }

    #[test]
    fn intersection_bad_data() -> rusqlite::Result<()> {
        let con = init_db()?;
        let r: rusqlite::Result<f64> = con.query_row(
            "SELECT HYPERMINHASH_INTERSECTION((SELECT X'00'), (SELECT X'00'))",
            rusqlite::params![],
            |row| row.get(0),
        );
        expect_error_msg(
            r,
            "IO-error in hyperminhash",
            "unpacked bad data without error",
        )
    }
}

#[cfg(not(feature = "serialize"))]
pub mod serialize_stub {
    use super::*;

    #[test]
    fn zero_returns_error() -> rusqlite::Result<()> {
        let con = init_db()?;
        let r: rusqlite::Result<Option<Vec<u8>>> =
            con.query_row("SELECT HYPERMINHASH_ZERO()", rusqlite::params![], |row| {
                row.get(0)
            });
        expect_error_msg(r, "`serialize`-feature", "error not reported: ")
    }

    #[test]
    fn serialize_returns_error() -> rusqlite::Result<()> {
        let con = init_db()?;
        let r: rusqlite::Result<Option<Vec<u8>>> = con.query_row(
            "SELECT HYPERMINHASH_SERIALIZE(name) FROM sqlite_master",
            rusqlite::params![],
            |row| row.get(0),
        );
        expect_error_msg(r, "`serialize`-feature", "error not reported: ")
    }

    #[test]
    fn deserialize_returns_error() -> rusqlite::Result<()> {
        let con = init_db()?;
        con.execute("CREATE TABLE foo (id INT)", rusqlite::params![])?;
        let r: rusqlite::Result<Option<f64>> = con.query_row(
            "SELECT HYPERMINHASH_DESERIALIZE(name) FROM sqlite_master",
            rusqlite::params![],
            |row| row.get(0),
        );
        expect_error_msg(r, "`serialize`-feature", "error not reported: ")
    }

    #[test]
    fn union_returns_error() -> rusqlite::Result<()> {
        let con = init_db()?;
        con.execute("CREATE TABLE foo (id INT)", rusqlite::params![])?;
        let r: rusqlite::Result<Option<Vec<u8>>> = con.query_row(
            "SELECT HYPERMINHASH_UNION(name, name) FROM sqlite_master",
            rusqlite::params![],
            |row| row.get(0),
        );
        expect_error_msg(r, "`serialize`-feature", "error not reported: ")
    }

    #[test]
    fn intersection_returns_error() -> rusqlite::Result<()> {
        let con = init_db()?;
        con.execute("CREATE TABLE foo (id INT)", rusqlite::params![])?;
        let r: rusqlite::Result<Option<f64>> = con.query_row(
            "SELECT HYPERMINHASH_INTERSECTION(name, name) FROM sqlite_master",
            rusqlite::params![],
            |row| row.get(0),
        );
        expect_error_msg(r, "`serialize`-feature", "error not reported: ")
    }
}
