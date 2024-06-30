use std::io::SeekFrom;

use wikipedia::{PageIndex, get_stream_from_file, get_pages_from_string};
use parse_wiki_text::{Configuration, Node, Parameter};
//use pandoc::{InputFormat, InputKind, OutputKind, OutputFormat, Pandoc, PandocOutput};


fn main() {
    let mut index = PageIndex::new();
    index.build_index_file("simplewiki-20240620-pages-articles-multistream-index.txt.bz2".to_string());
    let a = index.title_exists(String::from("Human")).unwrap();
    //println!("{:?}", a);
    let b = index.get_block_size(a.clone()).expect("awfaskdfm");
    //println!("{:?}", b);
    let c = get_stream_from_file(
        "simplewiki-20240620-pages-articles-multistream.xml.bz2".to_string(),
        Some(SeekFrom::Start(a.block_offset)),
        b
    );

    let pages = get_pages_from_string(c);
    let page = &pages[a.number_in_block as usize];
    //println!("{page:?}");

    /*let mut pd = Pandoc::new();
    pd.set_input_format(InputFormat::MediaWiki, Vec::new());
    pd.set_input(InputKind::Pipe(page.get_wikitext().to_string()));
    pd.set_output_format(OutputFormat::Html, Vec::new());
    pd.set_output(OutputKind::Pipe);
    let p = match pd.execute() {
        Ok(p) => p,
        Err(e) => panic!("{e}!")
    };
    match p {
        PandocOutput::ToBuffer(b) => println!("{b}"),
        _ => ()
    }*/
    
    let l = page.get_wikitext_fmt();
    let result = Configuration::default().parse(&l);
    assert!(result.warnings.is_empty());
    let p = parse_nodes(&result.nodes, true);
    println!("{p}");
}

fn parse_nodes(nodes: &Vec<Node>, do_refs: bool) -> String {
    let mut s: String = String::new();
    let mut bold: bool = false;
    let mut italic: bool = false;
    let mut comment: bool = false;
    let mut magic_word: bool = false;
    let mut references: String = String::new();
    let mut current_ref: u64 = 1;

    for node in nodes {
        match node {
            Node::Bold{ .. } => { bold = !bold; if bold { s += "<strong>"; } else { s += "</strong>"; } }
            Node::BoldItalic{ .. } => { bold = !bold; if bold { s += "<strong>"; } else { s += "</strong>"; }  
                                        italic = !italic; if italic { s += "<em>"; } else { s += "</em>"; } },
            Node::Category{ ordinal, target, .. } => { 
                if ordinal.is_empty() {
                    s += format!("<a href=\"/{target}\">{target}</a>").as_str();
                } else { 
                    s += format!("<a href=\"/{target}\">{}</a>", parse_nodes(ordinal, false)).as_str();
                } 
            },
            Node::CharacterEntity{ character, .. } => { s.push(*character); },
            //Node::Comment{ .. } => { comment = !comment; if comment { s += "<!--"; } else { s += "-->"; } },
            Node::DefinitionList{ items, .. } => { println!("!todo!: {items:?}"); },
            Node::EndTag{ name, .. } => { s += format!("</{name}>\n").as_str(); },
            Node::ExternalLink{ nodes, .. } => {
                let t = parse_nodes(nodes, false); 
                let u = t.split_whitespace().next().unwrap_or("");
                let w = t.replace(&(u.to_owned() + " "), "");
                // get only first word - thats the url
                s += &("<a href=\"".to_owned() + &u + "\">" + &w + "</a>") 
                },
            Node::Heading{ level, nodes, .. } => { s += format!("<h{level}>{}</h{level}>\n", parse_nodes(nodes, false)).as_str(); },
            Node::HorizontalDivider{ .. } => { s += "<hr>" },
            Node::Image{ target, text, .. } => {
                let text = parse_nodes(text, false);
                let args = text.split("|").collect::<Vec<_>>();
                if let Some(caption) = args.last() {
                    s += format!("<img src=\"{target}\" /> <p>{caption}</p>\n").as_str();
                } else {
                    s += format!("<img src=\"{target}\" />\n").as_str();
                }
            },
            Node::Italic{ .. } => { italic = !italic; if italic { s += "<em>"; } else { s += "</em>"; } },
            Node::Link{ target, text, .. } => { s += &("<a href=\"/".to_owned() + target.replace(" ", "_").as_str() + "\">" + parse_nodes(text, false).as_str() + "</a>") }, // change "/" to base url
            Node::MagicWord{ .. } => { magic_word = !magic_word; if magic_word { s += "<magic>"; } else { s += "</magic>"; } },
            Node::OrderedList{ items, .. } => {
                s += "<ol>\n";
                for i in items {
                    s += &("<li>".to_owned() + parse_nodes(&i.nodes, false).as_str() + "</li>\n");
                }
                s += "</ol>\n"
            },
            Node::ParagraphBreak{ .. } => { s += "<br><br>\n"; },
            Node::Parameter{ default, name, .. } => { println!("!todo!: {default:?}, {name:?}"); },
            Node::Preformatted{ nodes, .. } => { s += format!("{}\n", parse_nodes(nodes, false)).as_str(); },
            Node::Redirect{ target, .. } => { s += &("REDIRECT TO: ".to_owned() + target) },
            Node::StartTag{ name, .. } => { s += format!("<{name}>").as_str(); },
            Node::Table{ attributes, captions, rows, .. } => {
                println!("!todo!"); 
            },

            Node::Tag{ name, nodes, ..} => {
                let nodes = parse_nodes(nodes, false);
                if name == "ref" {
                    if do_refs {
                        references += format!("<li id=\"ref-{current_ref}\">{nodes}</li>\n ").as_str();
                        s += format!("<sup><a href=\"#ref-{current_ref}\">[{current_ref}]</a></sup> ").as_str();
                        current_ref += 1;
                    } else {
                        s += format!("<ref>{nodes}</ref>\n").as_str();
                    }
                } else {
                    println!("!todo tag!: {name:?}, {nodes}"); 
                }
            },

            Node::Template{ name, parameters, .. } => {
                let template = parse_nodes(name, false);
                let parameters = parse_parameters(parameters);
                match template.to_lowercase().as_str() {
                    "reflist" => {
                        let result = Configuration::default().parse(&references);
                        //println!("{:?}", result.warnings);
                        let references = parse_nodes(&result.nodes, false);
                        s += format!("<ol>\n{references}</ol>\n").as_str()
                    },
                    "main" => {
                        if let Some(u) = parameters.first() {
                            let u = &u.1;
                            s += format!("<em>See the main article: <a href=\"/{u}\">{u}</a></em><br><br>\n").as_str()
                        }
                    },
                    "height" => s+= format!("{}", parameters.iter().rev().map(|x| x.1.clone() + &x.0).collect::<Vec<_>>().join("")).as_str(),
                    "convert" => {
                        let p = parameters[..4].iter().map(|x| x.1.clone()).collect::<Vec<_>>().join("");
                        s += format!("{p}").as_str()
                    },
                    "clear" => s += "<br><br>\n",
                    "rp" => s += format!("<sup>:{}</sup>", parameters.iter().map(|x| x.1.clone()).collect::<Vec<_>>().join(" ")).as_str(),
                    _ => s += format!("{name:?}:<br> {}", &simple_tab(parameters)).as_str()
                }
                 
            },
            Node::Text{ value, .. } => {
                s += value;
            },
            Node::UnorderedList{ items, .. } => {
                s += "<ul>\n";
                for i in items {
                    s += &("<li>".to_owned() + parse_nodes(&i.nodes, false).as_str() + "</li>\n");
                }
                s += "</ul>\n"
            },
            _ => ()
        }
    }
    s
}

fn parse_parameters(parameters: &Vec<Parameter>) -> Vec<(String, String)> {
    let mut s: Vec<(String, String)> = Vec::new();
    for p in parameters {
        if let Some(name) = &p.name {
            let name = parse_nodes(&name, false);
            s.push((name.to_string(), parse_nodes(&p.value, false).to_string()));
        } else {
            s.push((String::new(), parse_nodes(&p.value, false).to_string()));
        }
    }
    s
}

fn simple_tab(params: Vec<(String, String)>) -> String {
    fn find(params: &Vec<(String, String)>, thing: &str) -> String {
        params.iter().find(|p| p.0 == thing).unwrap_or(&(String::new(), String::new())).1.clone()
    }
    let mut s = String::new();
    //let url: String = find(&params, "url");
    for mut p in params {
        if (p.1.contains("http://") || p.1.contains("https://")) && !p.1.contains("<") { p.1 = format!("<a href=\"{}\">{}</a>", p.1, p.1); }
        if p.0 == "image" || p.1.to_lowercase().contains(".jpg") || p.1.to_lowercase().contains(".png") { p.1 = format!("<img src=\"{}\"/>", p.1) }
        s += format!("{}: {}<br>\n", p.0, p.1).as_str();
    }
    s
}
