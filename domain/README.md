# `Domain` crate for validator/sentry stack

This crate is meant to hold all Domain structures required for both the Validator and Sentry.
It defines all the structs(Entities, Value Objects and etc.) and Repository traits and Domain errors.
It also defines the serialization and deserialization of them.
The actual implementations of Repositories is left to the underlying usage, whether it is
using Postgres, some sort of Memory implementation and etc.

### Features:

#### Fixtures

For testing purposes there are set of fixtures found under the `fixtures` module,
to easily create valid objects in UnitTests.

There is also the `domain::test_util` module that gives you some handy functions
for usage in tests:

- `take_one` which gives you a random element from a slice.


### Utils

The `domain::util` has some domain utilities.

- `ts_milliseconds_option` allows you to have `Option<DateTime<Utc>>` that is
(de)serialized into Milliseconds Timestamp.