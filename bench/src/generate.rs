use rand::Rng;

fn main() {
    let count = 100000;

    // length 20-500 random string
    let mut rng = rand::rng();

    let mut lines = vec![];

    for _ in 0..count {
        // 1000개마다 진행상황 출력
        if lines.len() % 10000 == 0 {
            println!("Generated {} lines", lines.len());
        }

        // uuid v4
        let key = uuid::Uuid::new_v4();

        // length 20-500 random string
        let length = rng.random_range(20..=200);
        let value = generate_random_string(length);

        let line = format!("{},{}", key, value);

        lines.push(line);
    }

    let csv_file_text = lines.join("\n");

    std::fs::write("dataset.csv", csv_file_text).unwrap();
}

fn generate_random_string(length: usize) -> String {
    use rand::Rng;

    let mut rng = rand::rng();
    (0..length)
        .map(|_| rng.random_range(b'a'..=b'z') as char)
        .collect()
}
