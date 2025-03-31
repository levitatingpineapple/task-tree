use markdown::{ParseOptions, to_mdast};

mod event;
mod session;

static TEST: &str = "
- [x] Parse repeat `25/03/29_19:00-20:00` `25/03/30_09:15-11:00`
    - Foo
    - Bar
";

fn main() -> Result<(), markdown::message::Message> {
    let ast = to_mdast(TEST, &ParseOptions::gfm())?;
    println!("{:#?}", ast);
    Ok(())
}
