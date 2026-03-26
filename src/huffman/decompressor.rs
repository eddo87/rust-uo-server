use super::huffman_table;

const TERMINAL_CODE_BIT_COUNT: u8 = 4;
const TERMINAL_CODE_VALUE: u32 = 0xD;

/// A node in the Huffman decode tree.
/// Internal nodes have children; leaf nodes hold the decoded byte value.
enum Node {
    Internal {
        /// Child when the next bit is 0
        zero: Option<Box<Node>>,
        /// Child when the next bit is 1
        one: Option<Box<Node>>,
    },
    /// Leaf node representing a decoded byte value.
    /// Uses u16 to accommodate values 0-255 for data bytes plus 256 for the terminal code.
    Leaf(u16),
}

/// Value used to represent the terminal code in the decode tree.
const TERMINAL_LEAF_VALUE: u16 = 256;

/// Builds a static Huffman decode tree from the compression table.
///
/// For each byte value 0-255, the table provides the compressed bit pattern and its length.
/// We also insert the terminal code (value=0xD, 4 bits) as leaf value 256.
///
/// The tree is constructed so that reading bits from MSB to LSB of the code pattern
/// navigates from the root to the correct leaf.
fn build_decode_tree() -> Node {
    let mut root = Node::Internal {
        zero: None,
        one: None,
    };

    // Insert all 256 byte values
    for byte_val in 0u16..=255 {
        let code = huffman_table::get_compressed_value(byte_val as u8);
        let bit_count = huffman_table::get_compressed_value_bit_count(byte_val as u8);
        insert_code(&mut root, code, bit_count, byte_val);
    }

    // Insert the terminal code as leaf value 256
    insert_code(
        &mut root,
        TERMINAL_CODE_VALUE,
        TERMINAL_CODE_BIT_COUNT,
        TERMINAL_LEAF_VALUE,
    );

    root
}

/// Inserts a code into the decode tree.
///
/// The code's bits are read from the most significant bit (bit_count-1) down to bit 0.
/// This matches the compressor's `write_bits` which shifts bits in from the right,
/// meaning the first bit written is the most significant bit of the code.
fn insert_code(root: &mut Node, code: u32, bit_count: u8, leaf_value: u16) {
    let mut current = root;

    for i in (0..bit_count).rev() {
        let bit = (code >> i) & 1;

        current = match current {
            Node::Internal {
                ref mut zero,
                ref mut one,
            } => {
                let child = if bit == 0 { zero } else { one };
                if child.is_none() {
                    *child = Some(Box::new(Node::Internal {
                        zero: None,
                        one: None,
                    }));
                }
                child.as_mut().unwrap().as_mut()
            }
            Node::Leaf(_) => {
                panic!(
                    "Huffman tree conflict: tried to traverse through a leaf node \
                     while inserting code for value {}",
                    leaf_value
                );
            }
        };
    }

    // Replace the current node with a leaf
    *current = Node::Leaf(leaf_value);
}

pub struct Decompressor {
    data: Vec<u8>,
}

impl Decompressor {
    pub fn new(data: Vec<u8>) -> Self {
        Decompressor { data }
    }

    /// Decompress the stored data by reading bits and walking the Huffman decode tree.
    ///
    /// Returns the decompressed bytes. Stops when the terminal code is encountered
    /// or when all input bits have been consumed.
    pub fn decompress(&self) -> Vec<u8> {
        let tree = build_decode_tree();
        let mut output = Vec::new();

        // Bit reader state: current byte index and bit position within that byte.
        // We read bits from MSB (bit 7) to LSB (bit 0) within each byte,
        // matching the compressor's output order.
        let mut byte_index: usize = 0;
        let mut bit_position: u8 = 7; // start at MSB of first byte

        loop {
            // Walk the tree from the root
            let mut node = &tree;

            loop {
                match node {
                    Node::Leaf(value) => {
                        if *value == TERMINAL_LEAF_VALUE {
                            return output;
                        }
                        output.push(*value as u8);
                        break;
                    }
                    Node::Internal { zero, one } => {
                        // Read the next bit
                        if byte_index >= self.data.len() {
                            // No more input data; return what we have
                            return output;
                        }

                        let bit = (self.data[byte_index] >> bit_position) & 1;

                        // Advance to next bit
                        if bit_position == 0 {
                            byte_index += 1;
                            bit_position = 7;
                        } else {
                            bit_position -= 1;
                        }

                        // Navigate the tree
                        let child = if bit == 0 { zero } else { one };
                        match child {
                            Some(child_node) => {
                                node = child_node.as_ref();
                            }
                            None => {
                                // Invalid bit sequence; stop decompression
                                return output;
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::huffman;

    /// Helper: compress data using the existing compressor, then decompress and compare.
    fn roundtrip(input: &[u8]) -> Vec<u8> {
        let mut compressed = Vec::new();
        huffman::compress(input.to_vec(), &mut compressed);
        let decompressor = Decompressor::new(compressed);
        decompressor.decompress()
    }

    #[test]
    fn roundtrip_empty_input() {
        let result = roundtrip(&[]);
        assert_eq!(result, vec![]);
    }

    #[test]
    fn roundtrip_single_byte() {
        let result = roundtrip(&[0x00]);
        assert_eq!(result, vec![0x00]);
    }

    #[test]
    fn roundtrip_single_byte_0x41() {
        // 'A'
        let result = roundtrip(&[0x41]);
        assert_eq!(result, vec![0x41]);
    }

    #[test]
    fn roundtrip_multiple_bytes() {
        let input = vec![0x01, 0x02, 0x03, 0x04, 0x05];
        let result = roundtrip(&input);
        assert_eq!(result, input);
    }

    #[test]
    fn roundtrip_repeated_bytes() {
        let input = vec![0xAA; 20];
        let result = roundtrip(&input);
        assert_eq!(result, input);
    }

    #[test]
    fn roundtrip_all_256_byte_values() {
        let input: Vec<u8> = (0..=255).collect();
        let result = roundtrip(&input);
        assert_eq!(result, input);
    }

    #[test]
    fn roundtrip_all_256_byte_values_reversed() {
        let input: Vec<u8> = (0..=255).rev().collect();
        let result = roundtrip(&input);
        assert_eq!(result, input);
    }

    #[test]
    fn roundtrip_longer_data() {
        let input: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        let result = roundtrip(&input);
        assert_eq!(result, input);
    }

    #[test]
    fn roundtrip_typical_packet_data() {
        // Simulate a typical UO packet: command byte followed by structured data
        let input = vec![
            0x80, 0x00, 0x00, 0x00, 0x00, 0x61, 0x64, 0x6D, 0x69, 0x6E, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00,
        ];
        let result = roundtrip(&input);
        assert_eq!(result, input);
    }

    #[test]
    fn decompress_empty_data() {
        let decompressor = Decompressor::new(vec![]);
        let result = decompressor.decompress();
        assert_eq!(result, vec![]);
    }

    #[test]
    fn decode_tree_is_consistent_with_compression_table() {
        // Verify that every byte value's Huffman code leads to the correct leaf
        // by manually walking the tree for each code.
        let tree = build_decode_tree();

        for byte_val in 0u16..=255 {
            let code = huffman_table::get_compressed_value(byte_val as u8);
            let bit_count = huffman_table::get_compressed_value_bit_count(byte_val as u8);

            let mut node = &tree;
            for i in (0..bit_count).rev() {
                let bit = (code >> i) & 1;
                match node {
                    Node::Internal { zero, one } => {
                        let child = if bit == 0 { zero } else { one };
                        node = child
                            .as_ref()
                            .unwrap_or_else(|| {
                                panic!("Missing child node for byte value {}", byte_val)
                            })
                            .as_ref();
                    }
                    Node::Leaf(_) => {
                        panic!(
                            "Reached leaf too early while walking code for byte value {}",
                            byte_val
                        );
                    }
                }
            }

            match node {
                Node::Leaf(value) => {
                    assert_eq!(
                        *value, byte_val,
                        "Leaf value mismatch for byte {}",
                        byte_val
                    );
                }
                _ => {
                    panic!(
                        "Did not reach a leaf after walking all bits for byte value {}",
                        byte_val
                    );
                }
            }
        }
    }
}
