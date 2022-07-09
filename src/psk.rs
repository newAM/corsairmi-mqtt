use anyhow::Context;
use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufRead},
    str::Split,
};

pub fn load(path: &str) -> anyhow::Result<HashMap<String, Vec<u8>>> {
    let file: File = File::open(path).with_context(|| format!("Failed to open PSK file {path}"))?;

    let lines: io::Lines<_> = io::BufReader::new(file).lines();

    let mut psks: HashMap<String, Vec<u8>> = HashMap::new();
    for (idx, line) in lines.enumerate() {
        let line: String = line.with_context(|| format!("Failed to read line {idx}"))?;
        let mut split: Split<char> = line.split(':');

        let identity: &str = split
            .next()
            .with_context(|| format!("{path}:{idx} Failed to deserialize identity from PSK"))?;

        if identity.is_empty() || identity.len() > 23 {
            anyhow::bail!("{path}:{idx} identity must be non-empty and 23 or fewer bytes");
        }

        let key: Vec<u8> = hex::decode(split.next().unwrap())
            .with_context(|| format!("{path}:{idx} Pre-shared key is not hex encoded"))?;

        if key.is_empty() || identity.len() > 256 {
            anyhow::bail!("{path}:{idx} key must be non-empty and 256 or fewer bytes");
        }

        if psks.insert(identity.to_string(), key).is_some() {
            log::warn!("{path}:{idx} duplicate PSK identity {identity}")
        }
    }

    if psks.is_empty() {
        anyhow::bail!("PSK file must contain at least one PSK")
    } else {
        Ok(psks)
    }
}

#[cfg(test)]
mod tests {
    use super::load;
    use std::{
        collections::HashMap,
        fs::{remove_file, File},
        io::Write,
    };

    const PATH: &str = "/tmp/corsairmi-mqtt-tests";

    #[test]
    fn basic() {
        remove_file(PATH).ok();

        {
            let mut file: File = File::create(PATH).unwrap();
            file.write_all(b"helloworld:deadbeef").unwrap();
        }

        let psks: HashMap<String, Vec<u8>> = load(PATH).unwrap();

        assert_eq!(psks.get("helloworld"), Some(&vec![0xde, 0xad, 0xbe, 0xef]))
    }
}
