export async function register() {
  if (process.env.NODE_ENV === "production" && !process.env.RUST_BACKEND_URL) {
    throw new Error(
      "RUST_BACKEND_URL is required in production. " +
      "Set it to the Rust backend URL (e.g., http://rust-backend:8080)."
    );
  }
}
