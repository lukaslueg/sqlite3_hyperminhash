import time
import sqlite3

if __name__ == "__main__":
    # Load the extension
    con = sqlite3.connect(":memory:")
    con.enable_load_extension(True)
    con.load_extension("target/release/libsqlite3_hyperminhash")

    # Create a dummy table
    con.execute('''CREATE TABLE foobar (foo INT NOT NULL, bar INT NOT NULL)''')
    con.executemany('''INSERT INTO foobar (foo, bar) VALUES (?, ?)''',
            ((i % 1231, i % 1409) for i in range(2000000)))

    # Real count
    t = time.perf_counter_ns()
    c = con.cursor()
    c.execute('''SELECT COUNT(*) FROM (SELECT DISTINCT foo, bar FROM foobar)''')
    real_count = c.fetchone()[0]
    real_t = time.perf_counter_ns() - t

    # Approximate count
    t = time.perf_counter_ns()
    c = con.cursor()
    c.execute('''SELECT hyperminhash(foo, bar) FROM foobar''')
    approx_count = c.fetchone()[0]
    approx_t = time.perf_counter_ns() - t

    print("%i unique rows in %.2fms via COUNT()" % (real_count, real_t / 1000000))
    print("%i unique rows (%.2f%% error) in %.2fms (%.1fx) via HYPERMINHASH()" %
            (approx_count,
             (1 - (approx_count / real_count)) * 100,
             approx_t / 1000000,
             real_t / approx_t))
