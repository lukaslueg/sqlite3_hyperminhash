use rand::Rng;

static AUTOLOAD: std::sync::Once = std::sync::Once::new();

fn init_db() -> rusqlite::Result<rusqlite::Connection> {
    AUTOLOAD.call_once(|| {
        // https://sqlite.org/c3ref/auto_extension.html
        let ptr = sqlite3_hyperminhash::sqlite3_sqlitehyperminhash_init
            as unsafe extern "C" fn(
                *mut std::ffi::c_void,
                *const std::ffi::c_void,
                *const std::ffi::c_void,
            ) -> i32;
        let rc = unsafe {
            sqlite3_hyperminhash::testutil::sqlite3_auto_extension(Some(std::mem::transmute(ptr)))
        };
        if rc as u32 != sqlite3_hyperminhash::testutil::SQLITE_OK {
            panic!("sqlite3_auto_extension failed");
        }
    });
    rusqlite::Connection::open_in_memory()
}

fn hmh_id(con: &rusqlite::Connection) -> rusqlite::Result<f64> {
    con.query_row(
        "SELECT hyperminhash(id) FROM foo",
        rusqlite::params![],
        |row| row.get(0),
    )
}

#[test]
fn empty_table() -> rusqlite::Result<()> {
    let con = init_db()?;
    con.execute("CREATE TABLE foo (id INT)", rusqlite::params![])?;
    // Count is zero
    assert_eq!(hmh_id(&con)?, 0.0);
    con.execute("INSERT INTO foo (id) VALUES (0)", rusqlite::params![])?;
    // Count is not zero
    let r = hmh_id(&con)?;
    assert!(r > 0.8);
    assert!(r < 1.2);
    Ok(())
}

#[test]
fn simple_count_error() -> rusqlite::Result<()> {
    let con = init_db()?;
    con.execute("CREATE TABLE foo (id INT)", rusqlite::params![])?;
    let mut stmt = con.prepare("INSERT INTO foo (id) VALUES (?1)")?;
    for i in 0..1000 {
        stmt.execute(&[i % 97])?;
    }
    let r = hmh_id(&con)?;
    // Error should be small
    assert!((1.0 - (r / 97.0)).abs() < 0.05);
    Ok(())
}

#[test]
fn data_types() -> rusqlite::Result<()> {
    let con = init_db()?;
    con.execute("CREATE TABLE bar (i INT, f FLOAT, s TEXT, b BLOB)", rusqlite::params![])?;
    con.execute("INSERT INTO bar (i, f, s, b) VALUES (?1, ?2, ?3, ?4)", rusqlite::params![1, 2.0, "3.0", &b"4.0"[..]])?;
    // All primitive data types are counted
    let r: f64 = con.query_row(
        "SELECT hyperminhash(i, f, s, b) FROM bar",
        rusqlite::params![],
        |row| row.get(0),
    )?;
    assert!(r > 0.8);
    assert!(r < 1.2);
    Ok(())
}

#[test]
fn random_data() -> rusqlite::Result<()> {
    let mut rnd = rand::thread_rng();
    let con = init_db()?;
    con.execute("CREATE TABLE bar (i INT, f FLOAT, s TEXT, b BLOB)", rusqlite::params![])?;
    let mut stmt = con.prepare("INSERT INTO bar (i, f, s, b) VALUES (?1, ?2, ?3, ?4)")?;
    for _ in 0..10_000 {
        let row: (i64, f64, [u8; 10]) = rnd.gen();
        let s: String = (0..10).map(|_| rnd.gen::<char>()).collect();
        stmt.execute(rusqlite::params![row.0, row.1, s, &row.2[..]])?;
    }
    con.execute("INSERT INTO bar (i, f, s, b) SELECT * FROM bar", rusqlite::params![])?;

    // Real count is 20000
    let r: i64 = con.query_row(
        "SELECT COUNT(*) FROM bar",
        rusqlite::params![],
        |row| row.get(0),
    )?;
    assert_eq!(r, 20_000);

    // Real distinct count is 10000
    let r: i64 = con.query_row(
        "SELECT COUNT(*) FROM (SELECT DISTINCT i, f, s, b FROM bar)",
        rusqlite::params![],
        |row| row.get(0),
    )?;
    assert_eq!(r, 10_000);

    // Approximate count has small error
    let r: f64 = con.query_row(
        "SELECT hyperminhash(i, f, s, b) FROM bar",
        rusqlite::params![],
        |row| row.get(0),
    )?;
    assert!((1.0 - (r / 10_000.0)).abs() < 0.05);
    Ok(())
}

#[test]
fn null_rows() -> rusqlite::Result<()> {
    let con = init_db()?;
    con.execute("CREATE TABLE foobar (foo INT, bar INT)", rusqlite::params![])?;
    let mut stmt = con.prepare("INSERT INTO foobar (foo, bar) VALUES (?1, ?2)")?;
    stmt.execute(rusqlite::params![Option::<i64>::None, Option::<i64>::None])?;
    stmt.execute(rusqlite::params![Option::<i64>::None, Option::<i64>::None])?;

    // Empty rows count as a single row
    let r: f64 = con.query_row(
        "SELECT hyperminhash(foo, bar) FROM foobar",
        rusqlite::params![],
        |row| row.get(0),
    )?;
    assert!(r > 0.8);
    assert!(r < 1.2);

    stmt.execute(rusqlite::params![&1, &2])?;
    let t: f64 = con.query_row(
        "SELECT hyperminhash(foo, bar) FROM foobar",
        rusqlite::params![],
        |row| row.get(0),
    )?;
    assert!(t > r);
    Ok(())
}

#[test]
fn null_data() -> rusqlite::Result<()> {
    let mut rnd = rand::thread_rng();
    let con = init_db()?;
    con.execute("CREATE TABLE foobar (foo INT, bar INT)", rusqlite::params![])?;
    let mut stmt = con.prepare("INSERT INTO foobar (foo, bar) VALUES (?1, ?2)")?;
    for _ in 0..10_000 {
        let row: (Option<u8>, Option<bool>) = rnd.gen();
        stmt.execute(rusqlite::params![row.0, row.1])?;
    }

    let real_count: f64 = con.query_row(
        "SELECT COUNT(*) FROM (SELECT DISTINCT foo, bar FROM foobar)",
        rusqlite::params![],
        |row| row.get(0),
    )?;
    let r: f64 = con.query_row(
        "SELECT hyperminhash(foo, bar) FROM foobar",
        rusqlite::params![],
        |row| row.get(0),
    )?;
    assert!((1.0 - (r / real_count)).abs() < 0.05);
    Ok(())
}
