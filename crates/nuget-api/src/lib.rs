use surf::Client;

#[derive(Debug)]
struct NuGetClient {
    key: String,
    client: Client
}
