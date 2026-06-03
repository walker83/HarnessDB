//! MongoDB wire protocol message formats
//! Reference: https://www.mongodb.com/docs/manual/reference/mongodb-wire-protocol/

use bytes::{Buf, BufMut, BytesMut};
use serde::{Deserialize, Serialize};
use std::io::{self, Cursor, Read, Write};

/// MongoDB message header (16 bytes)
#[derive(Debug, Clone)]
pub struct MessageHeader {
    pub message_length: i32,
    pub request_id: i32,
    pub response_to: i32,
    pub op_code: OpCode,
}

impl MessageHeader {
    pub const SIZE: usize = 16;

    pub fn parse(buf: &mut BytesMut) -> Option<Self> {
        if buf.len() < Self::SIZE {
            return None;
        }

        let message_length = buf.get_i32_le();
        let request_id = buf.get_i32_le();
        let response_to = buf.get_i32_le();
        let op_code_num = buf.get_i32_le();
        let op_code = OpCode::from_i32(op_code_num)?;

        Some(Self {
            message_length,
            request_id,
            response_to,
            op_code,
        })
    }

    pub fn encode(&self, buf: &mut BytesMut) {
        buf.put_i32_le(self.message_length);
        buf.put_i32_le(self.request_id);
        buf.put_i32_le(self.response_to);
        buf.put_i32_le(self.op_code.to_i32());
    }
}

/// MongoDB operation codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum OpCode {
    OP_REPLY = 1,
    OP_UPDATE = 2001,
    OP_INSERT = 2002,
    OP_QUERY = 2004,
    OP_GET_MORE = 2005,
    OP_DELETE = 2006,
    OP_KILL_CURSORS = 2007,
    OP_COMPRESSED = 2012,
    OP_MSG = 2013,
}

impl OpCode {
    pub fn from_i32(code: i32) -> Option<Self> {
        match code {
            1 => Some(OpCode::OP_REPLY),
            2001 => Some(OpCode::OP_UPDATE),
            2002 => Some(OpCode::OP_INSERT),
            2004 => Some(OpCode::OP_QUERY),
            2005 => Some(OpCode::OP_GET_MORE),
            2006 => Some(OpCode::OP_DELETE),
            2007 => Some(OpCode::OP_KILL_CURSORS),
            2012 => Some(OpCode::OP_COMPRESSED),
            2013 => Some(OpCode::OP_MSG),
            _ => None,
        }
    }

    pub fn to_i32(self) -> i32 {
        self as i32
    }
}

/// OP_MSG message (MongoDB 3.6+)
#[derive(Debug, Clone)]
pub struct OpMsg {
    pub flag_bits: u32,
    pub sections: Vec<Section>,
    pub checksum: Option<u32>,
}

/// OP_MSG section types
#[derive(Debug, Clone)]
pub enum Section {
    /// Body (kind 0) - Single BSON document
    Body(bson::Document),
    /// Document Sequence (kind 1) - Sequence of BSON documents
    DocumentSequence {
        identifier: String,
        documents: Vec<bson::Document>,
    },
}

impl OpMsg {
    pub fn parse(buf: &mut BytesMut) -> Option<Self> {
        if buf.len() < 5 {
            return None;
        }

        let flag_bits = buf.get_u32_le();
        let mut sections = Vec::new();

        while !buf.is_empty() {
            let kind = buf.get_u8();
            match kind {
                0 => {
                    // Body section - single BSON document
                    let mut cursor = Cursor::new(&buf[..]);
                    let doc = bson::Document::from_reader(&mut cursor).ok()?;
                    buf.advance(cursor.position() as usize);
                    sections.push(Section::Body(doc));
                    // NOTE: We do NOT break here. Some drivers (including pymongo)
                    // send kind 0 (body) BEFORE kind 1 (document sequences).
                    // Continue parsing to capture all sections.
                }
                1 => {
                    // Document sequence
                    let section_size = buf.get_i32_le() as usize;
                    let identifier_len = buf.iter().position(|&b| b == 0)?;
                    let identifier = String::from_utf8_lossy(&buf[..identifier_len]).to_string();
                    buf.advance(identifier_len + 1); // +1 for null terminator

                    let mut documents = Vec::new();
                    let remaining = section_size - identifier_len - 5;
                    if buf.len() >= remaining {
                        let mut docs_buf = buf.split_to(remaining);
                        while !docs_buf.is_empty() {
                            let mut cursor = Cursor::new(&docs_buf[..]);
                            if let Ok(doc) = bson::Document::from_reader(&mut cursor) {
                                let consumed = cursor.position() as usize;
                                docs_buf.advance(consumed);
                                documents.push(doc);
                            } else {
                                break;
                            }
                        }
                    }
                    sections.push(Section::DocumentSequence {
                        identifier,
                        documents,
                    });
                }
                _ => return None, // Unknown section kind
            }
        }

        Some(Self {
            flag_bits,
            sections,
            checksum: None,
        })
    }

    pub fn encode(&self, buf: &mut BytesMut) {
        buf.put_u32_le(self.flag_bits);

        for section in &self.sections {
            match section {
                Section::Body(doc) => {
                    buf.put_u8(0); // Kind 0
                    let mut doc_buf = Vec::new();
                    doc.to_writer(&mut doc_buf).ok();
                    buf.put_slice(&doc_buf);
                }
                Section::DocumentSequence {
                    identifier,
                    documents,
                } => {
                    buf.put_u8(1); // Kind 1

                    // Calculate section size
                    let identifier_bytes = identifier.as_bytes();
                    let mut docs_buf = Vec::new();
                    for doc in documents {
                        doc.to_writer(&mut docs_buf).ok();
                    }
                    let section_size = 4 + identifier_bytes.len() + 1 + docs_buf.len();

                    buf.put_i32_le(section_size as i32);
                    buf.put_slice(identifier_bytes);
                    buf.put_u8(0); // Null terminator
                    buf.put_slice(&docs_buf);
                }
            }
        }
    }

    /// Create a simple OP_MSG with a single body document
    pub fn new_body(doc: bson::Document) -> Self {
        Self {
            flag_bits: 0,
            sections: vec![Section::Body(doc)],
            checksum: None,
        }
    }
}

/// OP_QUERY message (legacy, but still used by some drivers)
#[derive(Debug, Clone)]
pub struct OpQuery {
    pub flags: u32,
    pub full_collection_name: String,
    pub number_to_skip: i32,
    pub number_to_return: i32,
    pub query: bson::Document,
    pub return_fields_selector: Option<bson::Document>,
}

impl OpQuery {
    pub fn parse(buf: &mut BytesMut) -> Option<Self> {
        if buf.len() < 12 {
            return None;
        }

        let flags = buf.get_u32_le();

        // Read null-terminated collection name
        let null_pos = buf.iter().position(|&b| b == 0)?;
        let full_collection_name = String::from_utf8_lossy(&buf[..null_pos]).to_string();
        buf.advance(null_pos + 1);

        if buf.len() < 8 {
            return None;
        }

        let number_to_skip = buf.get_i32_le();
        let number_to_return = buf.get_i32_le();

        let mut cursor = Cursor::new(&buf[..]);
        let query = bson::Document::from_reader(&mut cursor).ok()?;
        buf.advance(cursor.position() as usize);

        let return_fields_selector = if !buf.is_empty() {
            let mut cursor = Cursor::new(&buf[..]);
            let doc = bson::Document::from_reader(&mut cursor).ok();
            if doc.is_some() {
                buf.advance(cursor.position() as usize);
            }
            doc
        } else {
            None
        };

        Some(Self {
            flags,
            full_collection_name,
            number_to_skip,
            number_to_return,
            query,
            return_fields_selector,
        })
    }
}

/// OP_REPLY message
#[derive(Debug, Clone)]
pub struct OpReply {
    pub response_flags: u32,
    pub cursor_id: i64,
    pub starting_from: i32,
    pub number_returned: i32,
    pub documents: Vec<bson::Document>,
}

impl OpReply {
    pub fn encode(&self, buf: &mut BytesMut) {
        buf.put_u32_le(self.response_flags);
        buf.put_i64_le(self.cursor_id);
        buf.put_i32_le(self.starting_from);
        buf.put_i32_le(self.number_returned);

        for doc in &self.documents {
            let mut doc_buf = Vec::new();
            doc.to_writer(&mut doc_buf).ok();
            buf.put_slice(&doc_buf);
        }
    }

    pub fn new_ok(cursor_id: i64, documents: Vec<bson::Document>) -> Self {
        let number_returned = documents.len() as i32;
        Self {
            response_flags: 0,
            cursor_id,
            starting_from: 0,
            number_returned,
            documents,
        }
    }
}

/// Complete MongoDB message
#[derive(Debug, Clone)]
pub struct Message {
    pub header: MessageHeader,
    pub payload: MessagePayload,
}

#[derive(Debug, Clone)]
pub enum MessagePayload {
    OpMsg(OpMsg),
    OpQuery(OpQuery),
    OpReply(OpReply),
    Unknown(Vec<u8>),
}

impl Message {
    pub fn parse(buf: &mut BytesMut) -> Option<Self> {
        let mut header_buf = buf.clone();
        let header = MessageHeader::parse(&mut header_buf)?;

        if buf.len() < header.message_length as usize {
            return None; // Incomplete message
        }

        // Skip header
        buf.advance(MessageHeader::SIZE);

        let payload_len = header.message_length as usize - MessageHeader::SIZE;
        let mut payload_buf = buf.split_to(payload_len);

        let payload = match header.op_code {
            OpCode::OP_MSG => {
                MessagePayload::OpMsg(OpMsg::parse(&mut payload_buf)?)
            }
            OpCode::OP_QUERY => {
                MessagePayload::OpQuery(OpQuery::parse(&mut payload_buf)?)
            }
            _ => {
                MessagePayload::Unknown(payload_buf.to_vec())
            }
        };

        Some(Self { header, payload })
    }

    pub fn encode_reply(
        request_id: i32,
        response_to: i32,
        payload: MessagePayload,
    ) -> Self {
        let mut payload_buf = BytesMut::new();

        let op_code = match &payload {
            MessagePayload::OpMsg(msg) => {
                msg.encode(&mut payload_buf);
                OpCode::OP_MSG
            }
            MessagePayload::OpReply(reply) => {
                reply.encode(&mut payload_buf);
                OpCode::OP_REPLY
            }
            _ => OpCode::OP_MSG,
        };

        let message_length = MessageHeader::SIZE as i32 + payload_buf.len() as i32;

        let header = MessageHeader {
            message_length,
            request_id,
            response_to,
            op_code,
        };

        Self { header, payload }
    }

    pub fn encode(&self, buf: &mut BytesMut) {
        self.header.encode(buf);

        match &self.payload {
            MessagePayload::OpMsg(msg) => msg.encode(buf),
            MessagePayload::OpReply(reply) => reply.encode(buf),
            MessagePayload::OpQuery(_) => {} // Shouldn't encode queries in replies
            MessagePayload::Unknown(data) => buf.put_slice(data),
        }
    }
}
