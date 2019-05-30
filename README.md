# Hyperminhash for SQLite3

[![Build Status](https://travis-ci.org/lukaslueg/sqlite3_hyperminhash.svg?branch=master)](https://travis-ci.org/lukaslueg/sqlite3_hyperminhash)

A [Hyperminhash](https://github.com/lukaslueg/hyperminhash)-extension for SQLite3, providing very fast, constant-memory cardinality approximation, including intersection- and union-operations.

#### ... Query on an in-memory table of two million (INT, INT)-rows, no index

 Query                                                        | Result                  | Time
--------------------------------------------------------------|-------------------------|--------------
`SELECT COUNT(*) FROM (SELECT DISTINCT foo, bar FROM foobar)` | 1,734,479               | 5028ms
`SELECT hyperminhash(foo, bar) FROM foobar`                   | 1,728,632 (error 0.34%) | 337ms (x14.9)


## The extensions provides the following functions

* **`HYPERMINHASH()`**, an aggregate-function accepting up to `SQLITE_LIMIT_FUNCTION_ARG` arguments; returns the approximate cardinality of the items seen as a `DOUBLE`.

  E.g. `SELECT HYPERMINHASH(users.date, users.ip) AS unique_users FROM users;`

* **`HYPERMINHASH_ZERO()`**, a scalar-function accepting no arguments; returns a opaque `BLOB` representing a count of zero.

  E.g. `INSERT INTO stats (data_point, hmh_data) VALUES ('users', HYPERMINHASH_ZERO());`

* **`HYPERMINHASH_SERIALIZE()`**, an aggregate-function similar to `HYPERMINHASH()`. Returns a opaque `BLOB` representing the approximate cardinality of the items seen.

  E.g. `UPDATE stats SET stats.hmh_data = (SELECT HYPERMINHASH_SERIALIZE(users.date, users.ip) FROM users) WHERE stats.data_point = 'users';`

* **`HYPERMINHASH_DESERIALIZE()`**, a scalar-function accepting a single `BLOB` returned by `HYPERMINHASH_ZERO()`, `HYPERMINHASH_SERIALIZE()`, `HYPERMINHASH_ADD()` or `HYPERMINHASH_UNION()`. Returns the approximate cardinality as a `DOUBLE`.

  E.g. `SELECT HYPERMINHASH_DESERIALIZE(stats.hmh_data) FROM stats WHERE stats.data_point = 'users';`

* **`HYPERMINHASH_UNION()`**, an aggregate-function accepting `BLOB`s returned by `HYPERMINHASH_ZERO()`, `HYPERMINHASH_SERIALIZE()`, `HYPERMINHASH_ADD()` or `HYPERMINHASH_UNION()`. Returns an opaque `BLOB` representing the union-set operation over it's inputs.

  E.g. `SELECT HYPERMINHASH_UNION(stats.hmh_data) FROM stats WHERE stats.data_point = 'users' AND result = 'error';`

* **`HYPERMINHASH_ADD()`**, a scalar-function accepting up to `SQLITE_LIMIT_FUNCTION_ARG`, equivalent to `HYPERMINHASH_UNION()`.

  E.g. `UPDATE stats SET stats.hmh_data = HYPERMINHASH_ADD(stats.hmh_data, (SELECT HYPERMINHASH_SERIALIZE(users.date, users.ip) FROM users WHERE users.date = DATE('now'))) WHERE stats.data_point = 'users';`

* **`HYPERMINHASH_INTERSECTION()`**, a scalar-function accepting exactly two `BLOB`s returned by `HYPERMINHASH_ZERO()`, `HYPERMINHASH_SERIALIZE()`, `HYPERMINHASH_ADD()` or `HYPERMINHASH_UNION()`. Returns the approximate cardinality of the intersection-set operation over it's arguments as a `DOUBLE`.

  E.g. `SELECT HYPERMINHASH_INTERSECTION((SELECT stats.hmh_data FROM stats WHERE stats.data_point = 'users'), (SELECT stats.hmh_data FROM stats FROM stats WHERE stats.data_point = 'admins'));`

By default, only the `HYPERMINHASH()`-function is available. Compile the crate with the `serialize`-feature to enable the other functions, which return a static error if the `serialize`-feature was not activated.
