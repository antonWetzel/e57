use crate::paged_reader::PagedReader;
use crate::Error;
use crate::Header;
use crate::ReadSeek;
use std::io::Read;
use std::io::Seek;

pub struct E57 {
    reader: PagedReader,
    header: Header,
    xml: Vec<u8>,
}

impl E57 {
    /// Creates a new E57 instance for reading.
    pub fn new(mut reader: Box<dyn ReadSeek>) -> Result<Self, Error> {
        let mut header_bytes = [0_u8; 48];
        reader
            .read_exact(&mut header_bytes)
            .map_err(|_| Error::Read(String::from("Failed to read 48 byte file header")))?;

        // Parse and validate E57 header
        let header = Header::from_bytes(&header_bytes)?;

        // Set up paged reader for the CRC page layer
        let mut reader = PagedReader::new(reader, header.page_size)
            .map_err(|_| Error::InvalidFile(String::from("Unable to create paged reader")))?;

        // Read XML section
        reader
            .seek_physical(header.phys_xml_offset)
            .map_err(|_| Error::Read(String::from("Failed to seek to XML section")))?;
        let mut xml = vec![0_u8; header.xml_length as usize];
        reader
            .read_exact(&mut xml)
            .map_err(|_| Error::Read(String::from("Failed to read XML section")))?;

        Ok(Self {
            reader,
            header,
            xml,
        })
    }

    /// Returns the E57 file header structure.
    pub fn get_header(&self) -> Header {
        self.header.clone()
    }

    /// Returns the raw XML data of the E57 file as bytes.
    pub fn get_xml(&self) -> Vec<u8> {
        self.xml.clone()
    }

    /// Iterate over the whole file to check for CRC errors.
    pub fn validate_crc(&mut self) -> Result<(), Error> {
        self.reader.rewind().unwrap();
        let mut buffer = vec![0_u8; self.header.page_size as usize];
        while self
            .reader
            .read(&mut buffer)
            .map_err(|_| Error::Read(String::from("Failed to read file content")))?
            == 0
        {}
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header() {
        let file = std::fs::File::open("testdata/bunnyDouble.e57").unwrap();
        let reader = E57::new(Box::new(file)).unwrap();

        let header = reader.get_header();
        assert_eq!(header.major, 1);
        assert_eq!(header.minor, 0);
        assert_eq!(header.page_size, 1024);
    }

    #[test]
    fn xml() {
        let file = std::fs::File::open("testdata/bunnyDouble.e57").unwrap();
        let reader = E57::new(Box::new(file)).unwrap();
        let header = reader.get_header();
        let xml = reader.get_xml();
        assert_eq!(xml.len() as u64, header.xml_length);
        //std::fs::write("dump.xml", xml).unwrap();
    }

    #[test]
    fn validate() {
        let file = std::fs::File::open("testdata/bunnyDouble.e57").unwrap();
        let mut reader = E57::new(Box::new(file)).unwrap();
        reader.validate_crc().unwrap();
    }
}
