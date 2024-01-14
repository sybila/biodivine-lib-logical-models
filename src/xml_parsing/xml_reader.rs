use std::io::BufRead;

use xml::{reader::XmlEvent, EventReader};

pub trait XmlReader<BR: BufRead> {
    fn new(xml: EventReader<BR>) -> Self;
    fn next(&mut self) -> Result<XmlEvent, xml::reader::Error>;
}

impl<BR: BufRead> XmlReader<BR> for EventReader<BR> {
    fn new(xml: EventReader<BR>) -> Self {
        xml
    }

    #[inline]
    fn next(&mut self) -> Result<XmlEvent, xml::reader::Error> {
        EventReader::next(self)
    }
}

/// used for pretty printing of the read xml during the reading process
pub struct LoudReader<BR: BufRead> {
    xml: EventReader<BR>,
    curr_indent: usize,
}

impl<BR: BufRead> XmlReader<BR> for LoudReader<BR> {
    fn new(xml: EventReader<BR>) -> Self {
        Self {
            xml,
            curr_indent: 0,
        }
    }

    #[inline]
    fn next(&mut self) -> Result<XmlEvent, xml::reader::Error> {
        let event = self.xml.next()?;

        match event {
            XmlEvent::StartElement {
                ref name,
                // attributes,
                // namespace,
                ..
            } => {
                println!(
                    "{}<{:?}>",
                    (0..self.curr_indent).map(|_| ' ').collect::<String>(),
                    name
                );

                self.curr_indent += 2;
            }
            XmlEvent::EndElement { ref name, .. } => {
                self.curr_indent -= 2;
                println!(
                    "{}</{:?}>",
                    (0..self.curr_indent).map(|_| ' ').collect::<String>(),
                    name
                );
            }
            _ => {}
        };

        Ok(event)
    }
}

/// used for counting the line number of the read xml during the reading process
/// really mostly useless; just use `lines().count()` or smth
pub struct CountingReader<BR: BufRead> {
    xml: EventReader<BR>,
    pub curr_line: usize,
}

impl<BR: BufRead> XmlReader<BR> for CountingReader<BR> {
    fn new(xml: EventReader<BR>) -> Self {
        Self { xml, curr_line: 0 }
    }

    #[inline]
    fn next(&mut self) -> Result<XmlEvent, xml::reader::Error> {
        let event = self.xml.next()?;

        if matches!(
            event,
            XmlEvent::StartElement { .. } | XmlEvent::EndElement { .. }
        ) {
            self.curr_line += 1;
        }

        Ok(event)
    }
}
