use std::str::FromStr;
fn main() {
    let _s = <&'static str as FromStr>::from_str("hello").unwrap();
}
