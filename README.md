# Valord Map

## Overview

A dictionary sorted by values.

You can directly sort based on your data structure or implement OrdBy to specify sorting based on a particular field.

[![Crates.io][crates-badge]][crates-url]
[![docs.rs][docs-badge]][docs-url]
[![MIT licensed][mit-badge]][mit-url]

[crates-badge]: https://img.shields.io/crates/v/valord_map.svg
[crates-url]: https://crates.io/crates/valord-map
[docs-badge]: https://img.shields.io/docsrs/valord-map/latest
[docs-url]: https://docs.rs/valord-map
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/tediou5/valord-map/blob/master/LICENSE

## Example

```rust
use valord_map::{OrdBy, ValordMap};

#[derive(Debug, PartialEq)]
struct People {
    age: u8,
    name: String,
}

impl OrdBy for People {
    type Target = u8;

    fn ord_by<'a>(&'a self) -> &Self::Target {
        &self.age
    }
}

let mut peoples = ValordMap::new();
peoples.insert(
    1,
    People {
        age: 18,
        name: "qians1".to_string(),
    },
);
peoples.insert(
    2,
    People {
        age: 19,
        name: "qians2".to_string(),
    },
);
peoples.insert(
    3,
    People {
        age: 20,
        name: "qians3".to_string(),
    },
);
peoples.insert(
    4,
    People {
        age: 21,
        name: "qians4".to_string(),
    },
);
peoples.insert(
    5,
    People {
        age: 22,
        name: "qians5".to_string(),
    },
);

let youngest = peoples.first();
assert_eq!(youngest.len(), 1);
assert_eq!(
    youngest[0],
    (
        &1,
        &People {
            age: 18,
            name: "qians1".to_string(),
        }
    )
);

let oldest = peoples.last();
assert_eq!(oldest.len(), 1);
assert_eq!(
    oldest[0],
    (
        &5,
        &People {
            age: 22,
            name: "qians5".to_string(),
        }
    )
);

peoples
    .iter_mut()
    .for_each(|mut people_ref_mut| people_ref_mut.age += 1);

let youngest = peoples.first();
assert_eq!(youngest.len(), 1);
assert_eq!(
    youngest[0],
    (
        &1,
        &People {
            age: 19,
            name: "qians1".to_string(),
        }
    )
);

let oldest = peoples.last();
assert_eq!(oldest.len(), 1);
assert_eq!(
    oldest[0],
    (
        &5,
        &People {
            age: 23,
            name: "qians5".to_string(),
        }
    )
);

let range: Vec<_> = peoples.range(22..).collect();
assert_eq!(range.len(), 2);
println!("range: {range:?}");

let range: Vec<_> = peoples
    .range_mut(22..)
    .map(|mut rm_p| {
        let (k, v) = rm_p.get_mut_with_key();
        v.age = 30;
        (k.clone(), v.name.clone(), v.age)
    })
    .collect();

println!("range mut: {range:?}");

let oldest = peoples.last();
assert_eq!(oldest.len(), 2);
assert_eq!(
    oldest[0],
    (
        &4,
        &People {
            age: 30,
            name: "qians4".to_string(),
        }
    )
);
assert_eq!(
    oldest[1],
    (
        &5,
        &People {
            age: 30,
            name: "qians5".to_string(),
        }
    )
);
println!("peoples: {:?}", peoples.iter().collect::<Vec<_>>());
```
