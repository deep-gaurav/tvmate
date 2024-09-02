use rand::{distributions::Alphanumeric, Rng};

pub fn generate_random_string(length: usize) -> String {
    let mut rng = rand::thread_rng();
    let result: String = std::iter::repeat(())
        .map(|()| rng.sample(Alphanumeric) as char)
        .take(length)
        .collect();
    result.to_lowercase()
}
