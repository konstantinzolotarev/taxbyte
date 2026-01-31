/// Example demonstrating PostgreSQL User Repository usage
///
/// This example shows how to:
/// 1. Create a PostgreSQL connection pool
/// 2. Initialize the PostgresUserRepository
/// 3. Perform CRUD operations on users
/// 4. Handle errors properly
///
/// To run this example:
/// 1. Ensure PostgreSQL is running
/// 2. Set DATABASE_URL environment variable
/// 3. Run migrations: `sqlx migrate run`
/// 4. Execute: `cargo run --example user_repository_usage`
use sqlx::postgres::PgPoolOptions;
use taxbyte::domain::auth::{entities::User, ports::UserRepository, value_objects::Email};
use taxbyte::infrastructure::persistence::postgres::PostgresUserRepository;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Initialize tracing for better debugging
  tracing_subscriber::fmt::init();

  // Get database URL from environment
  let database_url = std::env::var("DATABASE_URL")
    .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/taxbyte".to_string());

  println!("Connecting to database...");

  // Create a connection pool
  let pool = PgPoolOptions::new()
    .max_connections(5)
    .connect(&database_url)
    .await?;

  println!("Connection pool created successfully");

  // Initialize the repository
  let repo = PostgresUserRepository::new(pool);

  println!("\n=== Creating a new user ===");

  // Create a new user
  let new_user = User::new(
    "alice@example.com".to_string(),
    "$argon2id$v=19$m=19456,t=2,p=1$...".to_string(), // Example hash
    "Alice Smith".to_string(),
  );

  match repo.create(new_user.clone()).await {
    Ok(created_user) => {
      println!("User created successfully!");
      println!("  ID: {}", created_user.id);
      println!("  Email: {}", created_user.email);
      println!("  Name: {}", created_user.full_name);
      println!("  Verified: {}", created_user.is_email_verified);

      println!("\n=== Finding user by ID ===");

      // Find user by ID
      match repo.find_by_id(created_user.id).await {
        Ok(Some(user)) => {
          println!("User found by ID!");
          println!("  Email: {}", user.email);
        }
        Ok(None) => println!("User not found"),
        Err(e) => println!("Error finding user: {}", e),
      }

      println!("\n=== Finding user by email ===");

      // Find user by email
      let email = Email::new("alice@example.com").unwrap();
      match repo.find_by_email(&email).await {
        Ok(Some(user)) => {
          println!("User found by email!");
          println!("  ID: {}", user.id);
          println!("  Name: {}", user.full_name);
        }
        Ok(None) => println!("User not found"),
        Err(e) => println!("Error finding user: {}", e),
      }

      println!("\n=== Updating user ===");

      // Update user
      let mut updated_user = created_user.clone();
      updated_user.update_full_name("Alice Johnson".to_string());

      match repo.update(updated_user).await {
        Ok(user) => {
          println!("User updated successfully!");
          println!("  New name: {}", user.full_name);
          println!("  Updated at: {}", user.updated_at);
        }
        Err(e) => println!("Error updating user: {}", e),
      }

      println!("\n=== Soft deleting user ===");

      // Soft delete user
      match repo.soft_delete(created_user.id).await {
        Ok(_) => println!("User soft deleted successfully!"),
        Err(e) => println!("Error deleting user: {}", e),
      }

      // Verify soft delete
      match repo.find_by_id(created_user.id).await {
        Ok(Some(user)) => {
          println!("User still exists after soft delete");
          println!(
            "  Email verification token cleared: {}",
            user.email_verification_token.is_none()
          );
          println!(
            "  Password reset token cleared: {}",
            user.password_reset_token.is_none()
          );
        }
        Ok(None) => println!("User not found after soft delete"),
        Err(e) => println!("Error checking deleted user: {}", e),
      }
    }
    Err(e) => {
      println!("Error creating user: {}", e);
    }
  }

  println!("\n=== Testing duplicate email constraint ===");

  // Try to create a user with duplicate email
  let duplicate_user = User::new(
    "alice@example.com".to_string(),
    "$argon2id$v=19$m=19456,t=2,p=1$...".to_string(),
    "Another Alice".to_string(),
  );

  match repo.create(duplicate_user).await {
    Ok(_) => println!("Unexpected: duplicate email was allowed"),
    Err(e) => println!("Expected error for duplicate email: {}", e),
  }

  println!("\n=== Example completed ===");

  Ok(())
}
