# Migaton

A SQLite database migration tool written in Rust.

_ _ _

## Purpose

Create a lightweight, rust-based, and easy-to-implement database migration tool.

## Implementation

You must first create the following directory structure (names other than .sql files do not matter, only hierarchy) to implement Migaton:

```
/migrations
	migaton.yml
	idx_table1unique
		chk.sql
		down.sql
		up.sql
	tbl_table1
		chk.sql
		down.sql
		up.sql
	tbl_table2
		chk.sql
		down.sql
		up.sql
	...etc
```

The config for `migaton.yml` is very straightforward at the moment. The *ordering* field contains an array which defines the order of migrations.

```yaml
ordering: [
  tbl_table1,
  idx_table1unique,
  tbl_table2,
]
```

The Rust code is also very straightforward. Migaton handles most of the heavy lifting for you. Here is an example from [Decibel DB's](https://github.com/frankiebaffa/decibel_db) `bin/migrate_up.rs` script.

```rust
// this is the connection which will run from memory for testing purposes.
// Migaton runs a full upward and downward migration in memory first, to verify that
// all migrations are valid SQL.
// The production database will not be migrated if any of these test migrations fail.
let mut mem_db = Database::init();
// attach the databases defined in the environment variable (specified in worm)
mem_db.context.attach_temp_dbs();
// this is the production db which will be migrated
let mut db = Database::init();
db.context.attach_dbs();
// get the connections from the contexts
let mut mem_c = mem_db.context.use_connection();
let mut c = db.context.use_connection();
// run migration upwards
let skips = match Migrator::do_up(&mut mem_c, &mut c, "./migrations") {
	Ok(res) => res,
	Err(e) => panic!("{}", e),
};
println!("{} migrations were skipped", skips);
```

