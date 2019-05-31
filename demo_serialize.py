import datetime
import random
import time
import sqlite3


def random_date():
    return datetime.date.fromtimestamp(random.randint(1451602800, 1546297199)).strftime('%Y-%m-%d')


def random_ipv6():
    return bytearray(random.getrandbits(8) for _ in range(3)) + b'\x00'*13


if __name__ == "__main__":

    ## Setup

    # Load the extension
    con = sqlite3.connect(":memory:")
    con.enable_load_extension(True)
    con.load_extension("target/release/libsqlite3_hyperminhash")

    # A log of users; date and ipv6
    con.execute('CREATE TABLE users (date DATE NOT NULL, ip BLOB(16) NOT NULL)');
    con.executemany('''INSERT INTO users (date, ip) VALUES (?, ?)''',
            ((random_date(), random_ipv6()) for i in range(250000)))

    con.execute('CREATE TABLE stats (data_point VARCHAR(255) PRIMARY KEY, hmh_data BLOB)')

    # Intialize a set with zero
    con.execute('INSERT INTO stats (data_point, hmh_data) VALUES ("users", HYPERMINHASH_ZERO())')

    def update_count():
        # Update the count, ran daily,
        # using `HYPERMINHASH_ADD` and `HYPERMINHASH_SERIALIZE`
        con.execute('''UPDATE stats
                       SET hmh_data =
                           HYPERMINHASH_ADD(
                               hmh_data,
                               (SELECT HYPERMINHASH_SERIALIZE(users.date, users.ip)
                                FROM users
                                WHERE users.date BETWEEN '2018-01-01' AND '2018-12-31')
                           )
                       WHERE stats.data_point = 'users'
                    ''')

    update_count()

    ## Usage

    # Current count via `HYPERMINHASH_DESERIALIZE()`
    c = con.cursor()
    c.execute('SELECT HYPERMINHASH_DESERIALIZE(hmh_data) FROM stats WHERE data_point = "users"')
    print("Current count is %i" % (c.fetchone()[0], ))

    # New days, more users...
    con.executemany('''INSERT INTO users (date, ip) VALUES (?, ?)''',
            ((random_date(), random_ipv6()) for i in range(250000)))

    update_count()

    # New count
    c = con.cursor()
    c.execute('SELECT HYPERMINHASH_DESERIALIZE(hmh_data) FROM stats WHERE data_point = "users"')
    print("Count is now %i" % (c.fetchone()[0], ))


    # Using `HYPERMINHASH_INTERSECTION` to get the count of unique users we've
    # seen 2017/2018 and before
    t = time.perf_counter_ns()
    c = con.cursor()
    c.execute('''SELECT HYPERMINHASH_INTERSECTION(
                          (SELECT HYPERMINHASH_SERIALIZE(users.ip)
                           FROM users
                           WHERE users.date BETWEEN '2017-01-01' AND '2018-12-31'),
                          (SELECT HYPERMINHASH_SERIALIZE(users.ip)
                           FROM users
                           WHERE users.date < '2017-01-01')
                        )
              ''')
    approx_t = time.perf_counter_ns() - t
    print("Recurring users, approx: %i, in %.2fms" % (c.fetchone()[0], approx_t / 1000000))


    # Same query using a subquery and DISTINCT
    t = time.perf_counter_ns()
    c = con.cursor()
    c.execute('''SELECT COUNT(DISTINCT users.ip)
                 FROM users
                 WHERE users.date BETWEEN '2017-01-01' AND '2018-12-31'
                 AND users.ip IN (SELECT u.ip
                                  FROM users AS u
                                  WHERE u.date < '2017-01-01')
              ''')
    approx_t = time.perf_counter_ns() - t
    print("Recurring users, exact: %i, in %.2fms" % (c.fetchone()[0], approx_t / 1000000))
