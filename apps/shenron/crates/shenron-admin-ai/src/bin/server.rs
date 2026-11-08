//! `shenron-admin-ai-server`: the cockpit hub. Configuration comes from the environment only
//! (see README): `SHENRON_ADMIN_AI_MODE`, `SHENRON_API_BASE_URL`, `SHENRON_ADMIN_AI_ADDR`, …

#[tokio::main]
async fn main() {
    let config = match shenron_admin_ai::server::Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("shenron-admin-ai: {e}");
            std::process::exit(2);
        }
    };
    if let Err(e) = shenron_admin_ai::server::serve(config).await {
        eprintln!("shenron-admin-ai: {e}");
        std::process::exit(1);
    }
}
