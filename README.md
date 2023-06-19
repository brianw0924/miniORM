A minimal ORM for Rust. 

### Usage
1. Clone this repository
2. new another project
3. specify the dependencies in `Cargo.toml`
```
[dependencies]
# Tokio:
sqlx = { version = "0.6", features = [ "runtime-tokio-native-tls" , "postgres" ] }
tokio = { version = "1", features = ["full"] }
miniORM = { path = "../miniORM" }
```
4. Sample code
```Rust
use miniORM::{CreateTable, Insert, Filter};
use sqlx::postgres::PgPoolOptions;
use sqlx::FromRow;

#[derive(Debug, FromRow, CreateTable, Insert, Filter)]
pub struct Person {
    name: String,
    age: i32,
}

async fn main() -> Result<(), sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://postgres:123@localhost").await?;

    // Create Table
    Person::create_table(&pool).await?;


    // Insert
    let brian = Person {
        name: String::from("brian"),
        age: 24,
    };

    let alice = Person {
        name: String::from("alice"),
        age: 24,
    };

    let bob = Person {
        name: String::from("bob"),
        age: 50,
    };

    println!("Insert\n{:#?}\n{:#?}\n{:#?}", brian, alice, bob);
    Person::insert(&pool, &brian).await?;
    Person::insert(&pool, &alice).await?;
    Person::insert(&pool, &bob).await?;

    // Select
    println!("Select all");
    let rows = Person::select(&pool).await?;
    for row in rows { 
        println!("age: {} | name: {}", row.age, row.name);
    }

    println!("Select age = 24");
    let rows = Person::filter().age(24).select(&pool).await?;
    for row in rows { 
        println!("age: {} | name: {}", row.age, row.name);
    }

    // Delete
    println!("Delete age = 24");
    Person::filter().age(24).delete(&pool).await?;
    let rows = Person::select(&pool).await?;
    for row in rows { 
        println!("age: {} | name: {}", row.age, row.name);
    }

    Ok(())
}
```