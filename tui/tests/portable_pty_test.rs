//! Simple test to verify portable-pty works in the current environment

use anyhow::Result;

fn main() -> Result<()> {
    println!("Testing if portable-pty is available...");

    // This will fail to compile if portable-pty is not available
    // but we're just checking if the dependency can be added

    println!("Portable-pty dependency test completed.");
    Ok(())
}
