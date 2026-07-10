mod logger;

#[tokio::main]
async fn main() {
    logger::init();
}
