use pulldown_cmark::{Event, Options, Parser};

fn main() {
    let md = "下面是要点：\n\n- 项目1\n- 项目2\n- 项目3";
    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES;
    let parser = Parser::new_ext(md, options);

    println!("=== Test 1: with blank line before list ===");
    for event in parser {
        println!("{:?}", event);
    }

    let md2 = "下面是要点：\n- 项目1\n- 项目2\n- 项目3";
    let parser2 = Parser::new_ext(md2, options);

    println!("\n=== Test 2: no blank line before list ===");
    for event in parser2 {
        println!("{:?}", event);
    }

    let md3 = "- 项目1\n- 项目2\n- 项目3";
    let parser3 = Parser::new_ext(md3, options);

    println!("\n=== Test 3: list only ===");
    for event in parser3 {
        println!("{:?}", event);
    }
}
