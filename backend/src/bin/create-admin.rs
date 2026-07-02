use std::env;

use backend::auth::password::hash_password;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("usage: create-admin <email> <display_name> <password>");
        std::process::exit(1);
    }
    let email = &args[1];
    let display_name = &args[2];
    let password = &args[3];

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("failed to connect to database");

    let password_hash = hash_password(password);

    sqlx::query(
        "INSERT INTO users (email, display_name, password_hash, is_admin)
         VALUES ($1, $2, $3, TRUE)
         ON CONFLICT (email) DO UPDATE
         SET display_name = EXCLUDED.display_name,
             password_hash = EXCLUDED.password_hash,
             is_admin = TRUE",
    )
    .bind(email)
    .bind(display_name)
    .bind(&password_hash)
    .execute(&pool)
    .await
    .expect("failed to upsert admin user");

    println!("Admin user '{email}' created/updated.");
}
