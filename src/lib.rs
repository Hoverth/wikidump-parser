use std::io::prelude::*;
use std::io::BufReader;
use std::io::SeekFrom;
use std::fs::File;

use bzip2::read::{BzDecoder, MultiBzDecoder};

use quick_xml::events::Event;
use quick_xml::reader::Reader;

fn u8_slice_to_string(slice: &[u8]) -> String {
    String::from_utf8(slice.to_vec()).expect("Invalid utf8!")
}

#[derive(Clone, Debug)]
pub struct PageIndexEntry {
    pub block_offset: u64,
    pub number_in_block: u64,
    pub page_id: u64,
    pub page_title: String
}

#[derive(Debug)]
pub struct PageIndex {
    pages: Vec<PageIndexEntry>
}

impl PageIndex {
    pub fn new() -> PageIndex {
        PageIndex {
            pages: Vec::new()
        }
    }

    pub fn build_index_file(&mut self, path: String){
        // open *-index.gz
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let mut decompressor = BzDecoder::new(reader);
        let mut contents: String = String::new();
        decompressor.read_to_string(&mut contents).unwrap();
        
        let mut index: Vec<PageIndexEntry> = Vec::new();
        let mut current_block: u64 = 0;
        let mut number_in_block: u64 = 0;

        // parse block:id:title
        for line in contents.lines() {
            let mut iter = line.split(":");
            let block_offset = iter.next().unwrap().parse().expect("error parsing block offset");
            if block_offset > current_block {
                number_in_block = 0;
                current_block = block_offset;
            }
            let page_id = iter.next().unwrap();
            let page_title = iter.next().unwrap();
            index.push(
                PageIndexEntry {
                    block_offset: block_offset, 
                    number_in_block: number_in_block,
                    page_id: page_id.parse().expect("error parsing page ID"), 
                    page_title: page_title.to_string()
                }
            );
            if block_offset == current_block {
                number_in_block += 1;
            }
        }
        // return Vec<pageindex> where pageindex {block_offset, id, title}
        self.pages = index;
    }
    
    pub fn id_exists(&mut self, id: u64) -> Option<PageIndexEntry> {
        for page in &self.pages {
            if page.page_id == id { return Some(page.clone()) }
        }
        None
    }

    pub fn title_exists(&mut self, title: String) -> Option<PageIndexEntry> {
        for page in &self.pages {
            if page.page_title == title { return Some(page.clone()) }
        }
        None
    }

    pub fn get_block_size(&mut self, target_page: PageIndexEntry) -> Option<u64> {
        let mut get_next_block: bool = false;
        for page in &self.pages {
            // this if statement assumes no id conflicts
            if page.page_id == target_page.page_id { get_next_block = true; } 
            if get_next_block {
                if page.block_offset > target_page.block_offset {
                    return Some(page.block_offset - target_page.block_offset);
                }
            }
        }
        None
    }
}


pub fn get_stream_from_file(path: String, pos: Option<SeekFrom>, len: u64) -> String {
    let file = File::open(path).unwrap();
    let mut reader = BufReader::new(file);
    if let Some(p) = pos {
        match reader.seek(p) {
            Err(e) => panic!("Error Seeking!: {e}"),
            _ => {}
        }
    }
    let mut decompressor = MultiBzDecoder::new(reader.take(len));
    let mut contents: String = String::new();
    decompressor.read_to_string(&mut contents).unwrap();
    contents
}

#[derive(Debug)]
pub struct Page {
    title: String,
    redirect: String,
    namespace: u64,
    id: u64,
    revision: Revision
}

impl Page {
    pub fn new() -> Page {
        Page {
        title: String::new(),
        redirect: String::new(),
        namespace: 0,
        id: 0,
        revision: Revision {
            id: 0,
            parent_id: 0,
            timestamp: String::new(),
            contributor: Contributor {
                username: String::new(),
                id: 0, 
                ip: String::new()
            },
            comment: String::new(),
            origin: 0,
            model: String::new(),
            format: String::new(),
            text: PageText {
                bytes: 0,
                sha1: String::new(),
                text: String::new()
            },
            sha1: String::new(),
        }
    }
    }

    pub fn get_wikitext(&self) -> &str {
        self.revision.text.text.as_str()
    }

    pub fn get_wikitext_fmt(&self) -> String {
        format!("={}= \n\n''{}''\n-----\n{}", self.title, self.revision.timestamp, self.revision.text.text)
    }
}

#[derive(Debug)]
pub struct Revision {
    id: u64,
    parent_id: u64,
    timestamp: String,
    contributor: Contributor,
    comment: String,
    origin: u64,
    model: String,
    format: String,
    text: PageText,
    sha1: String,
}

#[derive(Debug)]
pub struct Contributor {
    username: String,
    id: u64,
    ip: String,
}

#[derive(Debug)]
pub struct PageText {
    bytes: u64,
    sha1: String,
    text: String,
}

pub fn get_pages_from_string(string: String) -> Vec<Page> {
    let xml = string.as_str();
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut pages: Vec<Page> = Vec::new();
    
    let mut page: Page = Page::new();
    let mut parent_node: i8 = -1; // -1 -> err, 0 -> page, 1 -> revision, 2 -> contributor
    let mut parent_tag: String = String::new();
    
    loop {
        match reader.read_event_into(&mut buf) {
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            // exits the loop when reaching end of file
            Ok(Event::Eof) => break,
    
            Ok(Event::Start(e)) => {
                match e.name().as_ref() {
                    b"page" => { parent_node = 0; },
                    b"revision" => { parent_node = 1; },
                    b"contributor" => { parent_node = 2; },
                    b"text" => { 
                        /* also grab attr here - bytes & sha1 */ 
                        for a in e.attributes() {
                            match a {
                                Ok(a) => {
                                    match u8_slice_to_string(a.key.as_ref()).as_str() {
                                        "bytes" => { page.revision.text.bytes = u8_slice_to_string(a.value.as_ref()).parse().expect("ksdf"); },
                                        "sha1" => { page.revision.text.sha1 = u8_slice_to_string(a.value.as_ref()); },
                                        _ => ()
                                    }
                                },
                                Err(_) => ()
                            }
                        }
                        parent_tag = u8_slice_to_string(e.name().as_ref()); 
                    },
                    _ => { parent_tag = u8_slice_to_string(e.name().as_ref()); },
                }
            },
            Ok(Event::Empty(e)) => {
                match e.name().as_ref() {
                    b"redirect" => { 
                        /* fill in page.redirect with attr */ 
                        for a in e.attributes() {
                            match a {
                                Ok(a) => { page.redirect = String::from_utf8(a.value.as_ref().to_vec()).expect("Invalid utf8!"); },
                                Err(_) => ()
                            }
                        }
                    },
                    _ => ()
                }
            },
            Ok(Event::Text(e)) => {
                let text = String::from_utf8(e.as_ref().to_vec()).expect("Invalid utf8!");
                match parent_node {
                    0 => { // page
                        match parent_tag.as_str() {
                            "title" => { page.title = text; },
                            "ns" => { page.namespace = text.parse().expect("failed to parse number from string"); },
                            "id" => { page.id = text.parse().expect("failed to parse number from string"); },
                            _ => ()
                        }
                    },
                    1 => { // revision
                        match parent_tag.as_str() {
                            "id" => { page.revision.id = text.parse().expect("failed to parse number from string"); },
                            "parentId" => { page.revision.parent_id = text.parse().expect("failed to parse number from string"); },
                            "timestamp" => { page.revision.timestamp = text; },
                            "comment" => { page.revision.comment = text; },
                            "origin" => { page.revision.origin = text.parse().expect("failed to parse number from string"); },
                            "model" => { page.revision.model = text; },
                            "format" => { page.revision.format = text; },
                            "text" => { page.revision.text.text = html_escape::decode_html_entities(&text).to_string(); },
                            "sha1" => { page.revision.sha1 = text; },
                            _ => (),
                        }
                    },
                    2 => { // contributor
                        match parent_tag.as_str() {
                            "username" => { page.revision.contributor.username = text; },
                            "id" => { page.revision.contributor.id = text.parse().expect("failed to parse number from string"); },
                            "ip" => { page.revision.contributor.ip = text; },
                            _ => ()
                        }
                    }, 
                    _ => (),
                }
            },
            Ok(Event::End(e)) => {
                match e.name().as_ref() {
                    b"revision" | b"contributor" => { parent_node -= 1; },
                    b"page" => { 
                        parent_node = 0; 
                        pages.push(page);
                        page = Page::new(); 
                    }
                    _ => (),
                }
            }
    
            // There are several other `Event`s we do not consider here
            _ => (),
        }
        // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
        buf.clear();
    }
    pages
}
