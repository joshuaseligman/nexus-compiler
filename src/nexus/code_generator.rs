use log::*;

use crate::nexus::{syntax_tree::SyntaxTree, syntax_tree_node::*, symbol_table::*};
use crate::nexus::token::{TokenType, Keywords};
use crate::util::nexus_log;
use petgraph::graph::{NodeIndex};

use std::collections::HashMap;
use std::fmt;
use web_sys::{Document, Window, Element, DomTokenList};

enum CodeGenBytes {
    // Representation for final code/data in memory
    Code(u8),
    // Temporary variable address  until AST is traversed with identifier for later use
    Var(usize),
    // Temproary data for addition and boolean expression evaluation
    Temp(usize),
    // Spot is available for anything to take it
    Empty,
    // Represents data on the heap
    Data(u8),
    // This is a jump address for if and while statements
    Jump(usize)
}

// Customize the output when printing the string
impl fmt::Debug for CodeGenBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            CodeGenBytes::Code(code) => write!(f, "{:02X}", code),
            CodeGenBytes::Var(var) => write!(f, "V{}", var),
            CodeGenBytes::Temp(temp) => write!(f, "T{}", temp),
            CodeGenBytes::Empty => write!(f, "00"),
            CodeGenBytes::Data(data) => write!(f, "{:02X}", data),
            CodeGenBytes::Jump(jump) => write!(f, "J{}", jump)
        }
    }
}

// The struct for the code generator
#[derive (Debug)]
pub struct CodeGenerator {
    // The current max scope we have seen so far, which are encountered in
    // sequential order
    max_scope: usize,
    
    // The array for code gen
    code_arr: Vec<CodeGenBytes>,

    // The current location of the code in the memory array
    // The stack pointer is always code_pointer + 1
    code_pointer: u8,

    // The current location of the heap from the back of the array
    heap_pointer: u8,

    // The static table hashmap for <(id, scope), offset>
    static_table: HashMap<(String, usize), usize>,

    // Index for the temoprary data
    temp_index: usize,

    // Hashmap to keep track of the strings being stored on the heap
    string_history: HashMap<String, u8>,

    // Vector to keep track of each jump in the code
    jumps: Vec<u8>,

    // Flag for the memory being full
    is_memory_full: bool,
}

impl CodeGenerator {
    pub fn new() -> Self {
        let mut code_gen: CodeGenerator = CodeGenerator {
            // This is a flag for a new program
            max_scope: usize::MAX,

            // We are only able to store 256 bytes in memory
            code_arr: Vec::with_capacity(0x100),

            // Code starts at 0x00
            code_pointer: 0x00,

            // Heap starts at 0xFE (0xFF reserved for 0x00)
            heap_pointer: 0xFE,

            static_table: HashMap::new(),

            // Always start with a temp index of 0
            temp_index: 0,

            string_history: HashMap::new(),

            jumps: Vec::new(),

            is_memory_full: false
        };

        // Initialize the entire array to be unused spot in memory
        for i in 0..0x100 {
            code_gen.code_arr.push(CodeGenBytes::Empty);
        }

        return code_gen;
    }

    pub fn generate_code(&mut self, ast: &SyntaxTree, symbol_table: &mut SymbolTable, program_number: &u32) {
        debug!("Code gen called");

        // Make sure the current scope is set to be a flag for none
        self.max_scope = usize::MAX;
        
        // Reset the array and empty it out
        for i in 0..0x100 {
            self.code_arr[i] = CodeGenBytes::Empty;
        }

        self.code_pointer = 0x00;
        self.heap_pointer = 0xFE;

        self.static_table.clear();
        self.temp_index = 0;
        self.string_history.clear();
        self.jumps.clear();
        self.is_memory_full = false;

        // Generate the code for the program
        self.code_gen_block(ast, NodeIndex::new((*ast).root.unwrap()), symbol_table);
        // All programs end with 0x00, which is HALT
        self.add_code(0x00);
        debug!("{:?}", self.code_arr);

        self.backpatch_addresses();

        debug!("Static table: {:?}", self.static_table);
        debug!("Jumps vector: {:?}", self.jumps);
        debug!("{:?}", self.code_arr);

        self.display_code(program_number);
    }

    fn code_gen_block(&mut self, ast: &SyntaxTree, cur_index: NodeIndex, symbol_table: &mut SymbolTable) {
        // If this is the first block, then the first scope is 0
        if self.max_scope == usize::MAX {
            self.max_scope = 0;
        } else {
            // Otherwise just add 1
            self.max_scope += 1;
        }
        // Manually set the current scope because we are not able to look down
        // in the symbol table
        symbol_table.set_cur_scope(self.max_scope);

        // The current node is the block, so we need to loop through each of its children
        let neighbors: Vec<NodeIndex> = (*ast).graph.neighbors(cur_index).collect();

        for neighbor_index in neighbors.into_iter().rev() {
            debug!("{:?}", (*ast).graph.node_weight(neighbor_index).unwrap());
            let child: &SyntaxTreeNode = (*ast).graph.node_weight(neighbor_index).unwrap();
            
            match child {
                SyntaxTreeNode::NonTerminalAst(non_terminal) => {
                    match non_terminal {
                        NonTerminalsAst::Block => self.code_gen_block(ast, neighbor_index, symbol_table),
                        NonTerminalsAst::VarDecl => self.code_gen_var_decl(ast, neighbor_index, symbol_table),
                        NonTerminalsAst::Assign => self.code_gen_assignment(ast, neighbor_index, symbol_table),
                        NonTerminalsAst::Print => self.code_gen_print(ast, neighbor_index, symbol_table),
                        NonTerminalsAst::If => self.code_gen_if(ast, neighbor_index, symbol_table),
                        NonTerminalsAst::While => self.code_gen_while(ast, neighbor_index, symbol_table),
                        _ => error!("Received {:?} when expecting an AST nonterminal statement in a block", non_terminal)
                    }
                }
                _ => error!("Received {:?} when expecting an AST nonterminal for code gen in a block", child)
            }
        }

        // Exit the current scope
        symbol_table.end_cur_scope();
    }

    fn on_last_byte(&mut self) -> bool {
        return self.code_pointer == self.heap_pointer;
    }

    // Function to add byte of code to the memory array
    fn add_code(&mut self, code: u8) -> bool {
        if !self.is_memory_full {
            if self.on_last_byte() {
                // We are about the fill memory
                self.is_memory_full = true;
            }
            // Add the code to the next available spot in memory
            self.code_arr[self.code_pointer as usize] = CodeGenBytes::Code(code);
            self.code_pointer += 1;
            // No error, so successful addition to the code
            return true;
        } else {
            nexus_log::log(
                nexus_log::LogTypes::Error,
                nexus_log::LogSources::CodeGenerator,
                String::from("The stack has collided with the heap causing a stack overflow error")
            );
            return false;
        }
    }

    // Function to add byte of code to the memory array for variable addressing
    fn add_var(&mut self, var: usize) -> bool {
        if !self.is_memory_full {
            if self.on_last_byte() {
                // We are about the fill memory
                self.is_memory_full = true;
            }
            // Add the code to the next available spot in memory
            self.code_arr[self.code_pointer as usize] = CodeGenBytes::Var(var);
            self.code_pointer += 1;
            return true;
        } else {
            nexus_log::log(
                nexus_log::LogTypes::Error,
                nexus_log::LogSources::CodeGenerator,
                String::from("The stack has collided with the heap causing a stack overflow error")
            );
            return false;
        }
    }

    // Function to add byte of code to memory array for temporary data
    fn add_temp(&mut self, temp: usize) -> bool {
        if !self.is_memory_full {
            if self.on_last_byte() {
                // We are about the fill memory
                self.is_memory_full = true;
            }
            // Add the addressing for the temporary value
            self.code_arr[self.code_pointer as usize] = CodeGenBytes::Temp(temp);
            self.code_pointer += 1;
            return true;
        } else {
            nexus_log::log(
                nexus_log::LogTypes::Error,
                nexus_log::LogSources::CodeGenerator,
                String::from("The stack has collided with the heap causing a stack overflow error")
            );
            return false;
        }
    }

    // Function to add a byte of data to the heap
    fn add_data(&mut self, data: u8) -> bool {
        if !self.is_memory_full {
            if self.on_last_byte() {
                // We are about the fill memory
                self.is_memory_full = true;
            }
            // Heap starts from the end of the 256 bytes and moves towards the front
            self.code_arr[self.heap_pointer as usize] = CodeGenBytes::Data(data);
            self.heap_pointer -= 1;
            debug!("{:02X}; {:02X}", self.code_pointer, self.heap_pointer);
            return true;
        } else {
            nexus_log::log(
                nexus_log::LogTypes::Error,
                nexus_log::LogSources::CodeGenerator,
                String::from("The heap has collided with the stack causing a heap overflow error")
            );
            return false;
        }
    }

    fn store_string(&mut self, string: &str) -> Option<u8> {
        let addr: Option<&u8> = self.string_history.get(string);
        if addr.is_none() {
            // Assume the string gets stored
            let mut is_stored: bool = true;

            // All strings are null terminated, so start with a 0x00 at the end
            self.add_data(0x00);

            // Loop through the string in reverse order
            for c in string.chars().rev() {
                // Add the ascii code of each character
                if !self.add_data(c as u8) {
                    is_stored = false;
                    // Break if there was a heap overflow error
                    break;
                }
            }
           
            if is_stored {
                // Store it for future use
                self.string_history.insert(String::from(string), self.heap_pointer + 1);
                return Some(self.heap_pointer + 1);
            } else {
                // There is no address to return
                return None;
            }
        } else {
            // The string is already on the heap, so return its address
            return Some(*addr.unwrap());
        }
    }

    fn add_jump(&mut self) -> bool {
        if !self.is_memory_full {
            if self.on_last_byte() {
                // We are about the fill memory
                self.is_memory_full = true;
            }
            // Add the jump to the code and set it to 0 in the vector of jumps
            self.code_arr[self.code_pointer as usize] = CodeGenBytes::Jump(self.jumps.len());
            self.code_pointer += 1;
            self.jumps.push(0x00);
            return true;
        } else {
            nexus_log::log(
                nexus_log::LogTypes::Error,
                nexus_log::LogSources::CodeGenerator,
                String::from("The stack has collided with the heap causing a stack overflow error")
            );
            return false;
        }
    }

    // Replaces temp addresses with the actual position in memory
    fn backpatch_addresses(&mut self) -> bool { 
        // Determines where the memory is for the variables
        let mut var_pointer: u8 = self.code_pointer.to_owned();
        // Determines where the memory is for the temporary data
        let mut temp_pointer: u8 = self.heap_pointer.to_owned();
        for i in 0..self.code_arr.len() {
            match &self.code_arr[i] {
                CodeGenBytes::Var(offset) => {
                    // Get the variable address and make sure it is valid
                    let var_addr: u8 = self.code_pointer + *offset as u8;
                    if var_addr <= temp_pointer {
                        // If it is valid, update the array
                        self.code_arr[i] = CodeGenBytes::Code(var_addr);
                        if var_addr >= var_pointer {
                            // Update the pointer because we found a new variable
                            var_pointer = var_addr + 1;
                        }
                    } else {
                        // There was a collision
                        nexus_log::log(
                            nexus_log::LogTypes::Error,
                            nexus_log::LogSources::CodeGenerator,
                            String::from("The stack has collided with the heap causing a stack overflow error")
                        ); 
                        return false;
                    }
                },
                CodeGenBytes::Temp(offset) => {
                    let temp_addr: u8 = self.heap_pointer - *offset as u8;
                    if temp_addr >= var_pointer {
                        self.code_arr[i] = CodeGenBytes::Code(temp_addr);
                        if temp_addr <= temp_pointer {
                            // Update the max temp pointer
                            temp_pointer = temp_addr - 1;
                        }
                    } else {
                        // There was a collision
                        nexus_log::log(
                            nexus_log::LogTypes::Error,
                            nexus_log::LogSources::CodeGenerator,
                            String::from("The heap has collided with the stack causing a heap overflow error")
                        );
                        return false;
                    }
                },
                CodeGenBytes::Jump(jump_index) => {
                    self.code_arr[i] = CodeGenBytes::Code(self.jumps[*jump_index])
                }
                _ => {}
            }
        }
        return true;
    }

    // Function for creating the code for a variable declaration
    fn code_gen_var_decl(&mut self, ast: &SyntaxTree, cur_index: NodeIndex, symbol_table: &mut SymbolTable) {
        debug!("Code gen var decl");
        
        let children: Vec<NodeIndex> = (*ast).graph.neighbors(cur_index).collect();
        let id_node: &SyntaxTreeNode = (*ast).graph.node_weight(children[0]).unwrap();

        match id_node {
            SyntaxTreeNode::Terminal(token) => {
                debug!("{:?}; {:?}", token.text, symbol_table.cur_scope.unwrap());
                // Get the offset this variable will be on the stack
                let static_offset: usize = self.static_table.len();
                self.static_table.insert((token.text.to_owned(), symbol_table.cur_scope.unwrap()), static_offset);

                // Get the symbol table entry because strings have no code gen here, just the
                // static table entry
                let symbol_table_entry: &SymbolTableEntry = symbol_table.get_symbol(&token.text).unwrap();
                match symbol_table_entry.symbol_type {
                    Type::Int | Type::Boolean => {
                        // Generate the code for the variable declaration
                        self.add_code(0xA9);
                        self.add_code(0x00);
                        self.add_code(0x8D);
                        self.add_var(static_offset);
                        self.add_code(0x00);
                    },
                    Type::String => { /* Nothing to do here */ }
                }
            },
            _ => error!("Received {:?} when expecting terminal for var decl child in code gen", id_node)
        }
    }

    // Function for creating the code for an assignment
    fn code_gen_assignment(&mut self, ast: &SyntaxTree, cur_index: NodeIndex, symbol_table: &mut SymbolTable) {
        debug!("Code gen assignment");

        let children: Vec<NodeIndex> = (*ast).graph.neighbors(cur_index).collect();
        let value_node: &SyntaxTreeNode = (*ast).graph.node_weight(children[0]).unwrap();
        let id_node: &SyntaxTreeNode = (*ast).graph.node_weight(children[1]).unwrap();

        match value_node {
            SyntaxTreeNode::Terminal(token) => {
                match &token.token_type {
                    TokenType::Identifier(id_name) => {
                        debug!("Assignment id");
                        let value_id_entry: &SymbolTableEntry = symbol_table.get_symbol(&token.text).unwrap(); 
                        let value_static_offset: usize = self.static_table.get(&(token.text.to_owned(), value_id_entry.scope)).unwrap().to_owned();
                        
                        self.add_code(0xAD);
                        self.add_var(value_static_offset);
                        self.add_code(0x00);
                    },
                    TokenType::Digit(val) => {
                        debug!("Assignment digit");
                        // Digits just load a constant to the accumulator
                        self.add_code(0xA9);
                        self.add_code(*val as u8);
                    },
                    TokenType::Char(string) => {
                        debug!("Assignment string");
                        
                        // Start by storing the string
                        let addr: Option<u8> = self.store_string(&string);

                        // Store the starting address of the string in memory
                        if addr.is_some() {
                            self.add_code (0xA9);
                            self.add_code(addr.unwrap());
                        }
                    },
                    TokenType::Keyword(keyword) => {
                        match &keyword {
                            Keywords::True => {
                                debug!("Assignment true");
                                // True is 0x01
                                self.add_code(0xA9);
                                self.add_code(0x01);
                            },
                            Keywords::False => {
                                debug!("Assignment false");
                                // False is 0x00
                                self.add_code(0xA9);
                                self.add_code(0x00);
                            },
                            _ => error!("Received {:?} when expecting true or false for keyword terminals in assignment", keyword)
                        }
                    },
                    _ => error!("Received {:?} for terminal in assignment when expecting id, digit, char, or keyword", token)
                }
            },
            SyntaxTreeNode::NonTerminalAst(non_terminal) => {
                debug!("Assignment nonterminal");
                match non_terminal {
                    NonTerminalsAst::Add => {
                        // Call add, so the result will be in both the accumulator and in memory
                        self.code_gen_add(ast, children[0], symbol_table, true);
                    },
                    NonTerminalsAst::IsEq => {
                        self.code_gen_compare(ast, children[0], symbol_table, true);
                        self.get_z_flag_value();
                    },
                    NonTerminalsAst::NotEq => {
                        self.code_gen_compare(ast, children[0], symbol_table, false);
                        self.get_z_flag_value();
                    },
                    _ => error!("Received {:?} for nonterminal on right side of assignment for code gen", non_terminal)
                }
            },
            _ => error!("Received {:?} when expecting terminal or AST nonterminal for assignment in code gen", value_node)
        }

        match id_node {
            SyntaxTreeNode::Terminal(token) => {
                // Get the static offset for the variable being assigned to
                let id_entry: &SymbolTableEntry = symbol_table.get_symbol(&token.text).unwrap(); 
                let static_offset = self.static_table.get(&(token.text.to_owned(), id_entry.scope)).unwrap().to_owned();
                
                // The data that we are storing is already in the accumulator
                // so just run the code to store the data
                self.add_code(0x8D);
                self.add_var(static_offset);
                self.add_code(0x00);
            },
            _ => error!("Received {:?} when expecting terminal for assignmentchild in code gen", id_node)
        }
    }

    // Function for generating code for a print statement
    fn code_gen_print(&mut self, ast: &SyntaxTree, cur_index: NodeIndex, symbol_table: &mut SymbolTable) {
        debug!("Code gen print statement");

        // Get the child on the print statement to evaluate
        let children: Vec<NodeIndex> = (*ast).graph.neighbors(cur_index).collect();
        let child: &SyntaxTreeNode = (*ast).graph.node_weight(children[0]).unwrap();

        match child {
            SyntaxTreeNode::Terminal(token) => {
                match &token.token_type {
                    TokenType::Identifier(id_name) => {
                        let print_id: &SymbolTableEntry = symbol_table.get_symbol(&id_name).unwrap();
                        let static_offset: usize = self.static_table.get(&(id_name.to_owned(), print_id.scope)).unwrap().to_owned();
                        match &print_id.symbol_type {
                            Type::Int | Type::Boolean => {
                                debug!("Print id int/boolean");
                                
                                // Load the integer value into the Y register
                                self.add_code(0xAC);
                                self.add_var(static_offset);
                                self.add_code(0x00);

                                // Set X to 1 for the system call
                                self.add_code(0xA2);
                                self.add_code(0x01);
                            },
                            Type::String => {
                                debug!("Print id string");
                                // Store the string address in Y
                                self.add_code(0xAC);
                                self.add_var(static_offset);
                                self.add_code(0x00);

                                // X = 2 for this sys call
                                self.add_code(0xA2);
                                self.add_code(0x02);
                            },
                        }
                    },
                    TokenType::Digit(digit) => {
                        // Sys call 1 for integers needs the number in Y
                        self.add_code(0xA0);
                        self.add_code(*digit as u8);

                        // And X = 1
                        self.add_code(0xA2);
                        self.add_code(0x01);
                    },
                    TokenType::Char(string) => {
                        // Store the string in memory and load its address to Y
                        let addr: Option<u8> = self.store_string(&string);
                        if addr.is_some() {
                            self.add_code(0xA0);
                            self.add_code(addr.unwrap());
                        }

                        // X = 2 for a string sys call
                        self.add_code(0xA2);
                        self.add_code(0x02);
                    },
                    TokenType::Keyword(keyword) => {
                        self.add_code(0xA0);
                        match keyword {
                            Keywords::True => {
                                // Y = 1 for true
                                self.add_code(0x01);
                            },
                            Keywords::False => {
                                // Y = 0 for false
                                self.add_code(0x00);
                            },
                            _ => error!("Received {:?} when expecting true or false for print keyword", keyword)
                        }
                        // X = 1 for the sys call
                        self.add_code(0xA2);
                        self.add_code(0x01);
                    },
                    _ => error!("Received {:?} when expecting id, digit, string, or keyword for print terminal", token)
                }
            },
            SyntaxTreeNode::NonTerminalAst(non_terminal) => {
                debug!("Print nonterminal");
                match non_terminal {
                    NonTerminalsAst::Add => {
                        // Generate the result of the addition expression
                        self.code_gen_add(ast, children[0], symbol_table, true);

                        self.add_code(0x8D);
                        self.add_temp(self.temp_index);
                        self.temp_index += 1;
                        self.add_code(0x00);
                        
                        // Load the result to Y (wish there was TAY)
                        self.add_code(0xAC);
                        self.add_temp(self.temp_index - 1);
                        self.temp_index -= 1;
                        self.add_code(0x00);

                        // X = 1 for the sys call for integers
                        self.add_code(0xA2);
                        self.add_code(0x01);
                    },
                    NonTerminalsAst::IsEq => {
                        self.code_gen_compare(ast, children[0], symbol_table, true);
                        self.get_z_flag_value();

                        self.add_code(0x8D);
                        self.add_temp(self.temp_index);
                        self.temp_index += 1;
                        self.add_code(0x00);
                        
                        // Load the result to Y (wish there was TAY)
                        self.add_code(0xAC);
                        self.add_temp(self.temp_index - 1);
                        self.temp_index -= 1;
                        self.add_code(0x00);

                        // X = 1 for the sys call for integers
                        self.add_code(0xA2);
                        self.add_code(0x01);
                    },
                    NonTerminalsAst::NotEq => {
                        self.code_gen_compare(ast, children[0], symbol_table, false);
                        self.get_z_flag_value();

                        self.add_code(0x8D);
                        self.add_temp(self.temp_index);
                        self.temp_index += 1;
                        self.add_code(0x00);
                        
                        // Load the result to Y (wish there was TAY)
                        self.add_code(0xAC);
                        self.add_temp(self.temp_index - 1);
                        self.temp_index -= 1;
                        self.add_code(0x00);

                        // X = 1 for the sys call for integers
                        self.add_code(0xA2);
                        self.add_code(0x01);
                    },
                    _ => error!("Received {:?} when expecting addition or boolean expression for nonterminal print", non_terminal)
                }
            },
            _ => error!("Received {:?} when expecting terminal or AST nonterminal for print in code gen", child)
        }

        // The x and y registers are all set up, so just add the sys call
        self.add_code(0xFF);
    }

    // Function to generate code for an addition statement
    // Result is left in the accumulator
    fn code_gen_add(&mut self, ast: &SyntaxTree, cur_index: NodeIndex, symbol_table: &mut SymbolTable, first: bool) {
        debug!("Code gen add");

        // Get the child for addition
        let children: Vec<NodeIndex> = (*ast).graph.neighbors(cur_index).collect();
        let right_child: &SyntaxTreeNode = (*ast).graph.node_weight(children[0]).unwrap();
        let left_child: &SyntaxTreeNode = (*ast).graph.node_weight(children[1]).unwrap();

        match right_child {
            SyntaxTreeNode::Terminal(token) => {
                match &token.token_type {
                    TokenType::Digit(num) => {
                        // Store right side digit in the accumulator
                        self.add_code(0xA9);
                        self.add_code(*num);
                    },
                    TokenType::Identifier(id_name) => {
                        // Get the address needed from memory for the identifier
                        let value_id_entry: &SymbolTableEntry = symbol_table.get_symbol(&token.text).unwrap(); 
                        let value_static_offset: usize = self.static_table.get(&(token.text.to_owned(), value_id_entry.scope)).unwrap().to_owned();
                        
                        // Load the value into the accumulator
                        self.add_code(0xAD);
                        self.add_var(value_static_offset);
                        self.add_code(0x00);
                    },
                    _ => error!("Received {:?} when expecting digit or id for right side of addition", token)
                }

                // Both digits and ids are in the accumulator, so move them to
                // the res address for usage in the math operation
                self.add_code(0x8D);
                self.add_temp(self.temp_index);
                // We are using a new temporary value for temps, so increment the index
                self.temp_index += 1;
                self.add_code(0x00);
            },
            // Nonterminals are always add, so just call it
            SyntaxTreeNode::NonTerminalAst(non_terminal) => self.code_gen_add(ast, children[0], symbol_table, false),
            _ => error!("Received {:?} when expecting terminal or AST nonterminal for right addition value", right_child)
        }

        match left_child {
            SyntaxTreeNode::Terminal(token) => {
                match &token.token_type {
                    TokenType::Digit(num) => {
                        // Put left digit in acc
                        self.add_code(0xA9);
                        self.add_code(*num);

                        // Perform the addition
                        self.add_code(0x6D);
                        // Temp index - 1 is where the data is being stored
                        self.add_temp(self.temp_index - 1);
                        self.add_code(0x00);

                        // Only store the result back in memory if we have more addition to do
                        if !first {
                            // Store it back in the resulting address
                            self.add_code(0x8D);
                            self.add_temp(self.temp_index - 1);
                            self.add_code(0x00);
                        } else {
                            // We are done with the memory location, so can move
                            // the pointer back over 1
                            self.temp_index -= 1;
                        }
                    },
                    _ => error!("Received {:?} when expecting a digit for left side of addition for code gen", token)
                }
            },
            _ => error!("Received {:?} when expecting a terminal for the left side of addition for code gen", left_child)
        }
    }

    // Function to generate code for comparisons
    // Result is left in the Z flag and get_z_flag_vale function can be used
    // afterwards to place z flag value into the accumulator
    fn code_gen_compare(&mut self, ast: &SyntaxTree, cur_index: NodeIndex, symbol_table: &mut SymbolTable, is_eq: bool) {
        debug!("Code gen compare");

        // Get the child for comparison
        let children: Vec<NodeIndex> = (*ast).graph.neighbors(cur_index).collect();
        let right_child: &SyntaxTreeNode = (*ast).graph.node_weight(children[0]).unwrap();
        let left_child: &SyntaxTreeNode = (*ast).graph.node_weight(children[1]).unwrap();

        match left_child {
            SyntaxTreeNode::Terminal(token) => {
                match &token.token_type {
                    TokenType::Identifier(id_name) => {
                        // Get the address needed from memory for the identifier
                        let value_id_entry: &SymbolTableEntry = symbol_table.get_symbol(&token.text).unwrap(); 
                        let value_static_offset: usize = self.static_table.get(&(token.text.to_owned(), value_id_entry.scope)).unwrap().to_owned();
                        
                        // Load the value into the accumulator
                        self.add_code(0xAD);
                        self.add_var(value_static_offset);
                        self.add_code(0x00);
                    },
                    TokenType::Digit(num) => {
                        // Store the digit in memory
                        self.add_code(0xA9);
                        self.add_code(*num);
                    },
                    TokenType::Char(string) => {
                        let string_addr: Option<u8> = self.store_string(string);
                        if string_addr.is_some() {
                            self.add_code(0xA9);
                            self.add_code(string_addr.unwrap());
                        }
                    },
                    TokenType::Keyword(keyword) => {
                        self.add_code(0xA9);
                        let res: bool = match &keyword {
                            Keywords::True => self.add_code(0x01),
                            Keywords::False => self.add_code(0x00),
                            _ => {
                                error!("Received {:?} when expecting true or false for keywords in boolean expression", keyword);
                                false
                            }
                        };
                    },
                    _ => error!("Received {:?} when expecting an Id, digit, char, or keyword for left side of boolean expression", token)
                }
            },
            SyntaxTreeNode::NonTerminalAst(non_terminal) => {
                match &non_terminal {
                    NonTerminalsAst::Add => {
                        self.code_gen_add(ast, children[1], symbol_table, true);
                    },
                    NonTerminalsAst::IsEq => {
                        self.code_gen_compare(ast, children[1], symbol_table, true);
                        self.get_z_flag_value();
                    },
                    NonTerminalsAst::NotEq => {
                        self.code_gen_compare(ast, children[1], symbol_table, false);
                        self.get_z_flag_value();
                    },
                    _ => error!("Received {:?} for left side of nonterminal boolean expression, when expected Add, IsEq, or NotEq", non_terminal)
                }
            },
            _ => error!("Received {:?} when expected terminal or AST nonterminal for left side of comparison in code gen", left_child)
        }

        // The left hand side is already in the ACC, so can store in temp memory
        self.add_code(0x8D);
        self.add_temp(self.temp_index);
        self.temp_index += 1;
        self.add_code(0x00);

        match right_child {
            SyntaxTreeNode::Terminal(token) => {
                match &token.token_type {
                    TokenType::Identifier(id_name) => {
                        // Get the address needed from memory for the identifier
                        let value_id_entry: &SymbolTableEntry = symbol_table.get_symbol(&token.text).unwrap(); 
                        let value_static_offset: usize = self.static_table.get(&(token.text.to_owned(), value_id_entry.scope)).unwrap().to_owned();
                        
                        // Load the value into the X register
                        self.add_code(0xAE);
                        self.add_var(value_static_offset);
                        self.add_code(0x00);
                    },
                    TokenType::Digit(num) => {
                        // Store the digit in X
                        self.add_code(0xA2);
                        self.add_code(*num);
                    },
                    TokenType::Char(string) => {
                        let string_addr: Option<u8> = self.store_string(string);
                        if string_addr.is_some() {
                            self.add_code(0xA2);
                            self.add_code(string_addr.unwrap());
                        }
                    },
                    TokenType::Keyword(keyword) => {
                        self.add_code(0xA2);
                        let res: bool = match &keyword {
                            Keywords::True => self.add_code(0x01),
                            Keywords::False => self.add_code(0x00),
                            _ => {
                                error!("Received {:?} when expecting true or false for keywords in boolean expression", keyword);
                                false
                            }
                        };
                    },
                    _ => error!("Received {:?} when expecting an Id, digit, char, or keyword for left side of boolean expression", token)
                }
            },
            SyntaxTreeNode::NonTerminalAst(non_terminal) => {
                match &non_terminal {
                    NonTerminalsAst::Add => {
                        self.code_gen_add(ast, children[0], symbol_table, true);
                    },
                    NonTerminalsAst::IsEq => {
                        self.code_gen_compare(ast, children[0], symbol_table, true);
                        self.get_z_flag_value();
                    },
                    NonTerminalsAst::NotEq => {
                        self.code_gen_compare(ast, children[0], symbol_table, false);
                        self.get_z_flag_value();
                    },
                    _ => error!("Received {:?} for right side of nonterminal boolean expression, when expected Add, IsEq, or NotEq", non_terminal)
                }

                // The nonterminal result is in the ACC, so have to move to X
                self.add_code(0x8D);
                self.add_temp(self.temp_index);
                self.temp_index += 1;
                self.add_code(0x00);

                self.add_code(0xAE);
                self.add_temp(self.temp_index - 1);
                self.add_code(0x00);
                self.temp_index -= 1;
            },
            _ => error!("Received {:?} when expected terminal or AST nonterminal for left side of comparison in code gen", left_child)
        }

        self.add_code(0xEC);
        self.add_temp(self.temp_index - 1);
        self.add_code(0x00);

        // We are done with this data
        self.temp_index -= 1;

        // Add code if the operation is for not equals
        // This effectively flips the Z flag
        if !is_eq {
            // Start assuming that they were not equal
            self.add_code(0xA2);
            self.add_code(0x00);
            // Take the branch if not equal
            self.add_code(0xD0);
            self.add_code(0x02);
            // If equal, set x to 1
            self.add_code(0xA2);
            self.add_code(0x01);
            // Compare with 0 to flip the Z flag
            self.add_code(0xEC);
            self.add_code(0xFF);
            self.add_code(0x00);
        }
    }

    // Stores the value of the Z flag into the accumulator
    fn get_z_flag_value(&mut self) {
        // Assume Z is set to 0
        self.add_code(0xA9);
        self.add_code(0x00);
        // If it is 0, branch
        self.add_code(0xD0);
        self.add_code(0x02);
        // Otherwise, set the acc to 1
        self.add_code(0xA9);
        self.add_code(0x01);
    }

    fn code_gen_if(&mut self, ast: &SyntaxTree, cur_index: NodeIndex, symbol_table: &mut SymbolTable) {
        debug!("Code gen if");

        // Get the child for comparison
        let children: Vec<NodeIndex> = (*ast).graph.neighbors(cur_index).collect();
        let left_child: &SyntaxTreeNode = (*ast).graph.node_weight(children[1]).unwrap();

        // Starting address for the branch, but 0 will never be valid, so can have
        // default value set to 0
        let mut start_addr: u8 = 0x00;
        // This is the index of the jump that will ultimately be backpatched
        let jump_index: usize = self.jumps.len();

        match left_child {
            SyntaxTreeNode::NonTerminalAst(non_terminal) => {
                match &non_terminal {
                    // Evaluate the boolean expression for the if statement
                    // The Z flag is set by these function calls
                    NonTerminalsAst::IsEq => self.code_gen_compare(ast, children[1], symbol_table, true),
                    NonTerminalsAst::NotEq => self.code_gen_compare(ast, children[1], symbol_table, false),
                    _ => error!("Received {:?} when expecting IsEq or NotEq for nonterminal if expression", non_terminal)
                }
                // Add the branch code
                self.add_code(0xD0);
                self.add_jump();
                start_addr = self.code_pointer.to_owned();
            },
            SyntaxTreeNode::Terminal(token) => {
                match &token.token_type {
                    TokenType::Keyword(Keywords::True) => {/* Small optimization because no comparison is needed */}
                    TokenType::Keyword(Keywords::False) => {
                        // No code should be generated here because the if-statement is just dead
                        // code and will never be reached, so no point in trying to store the code
                        // with the limited space that we already have (256 bytes)
                        return;
                    }
                    _ => error!("Received {:?} when expecting true or false for if expression terminals", token)
                }
            },
            _ => error!("Received {:?} when expecting AST nonterminal or a terminal", left_child)
        }

        // Generate the code for the body
        self.code_gen_block(ast, children[0], symbol_table);

        // If there was a comparison to make, there is a start addr
        if start_addr != 0x00 {
            // Compute the difference and set it in the vector for use in backpatching
            let branch_offset: u8 = self.code_pointer - start_addr;
            self.jumps[jump_index] = branch_offset;
        }
    }

    fn code_gen_while(&mut self, ast: &SyntaxTree, cur_index: NodeIndex, symbol_table: &mut SymbolTable) {
        debug!("Code gen while");

        // Get the child for comparison
        let children: Vec<NodeIndex> = (*ast).graph.neighbors(cur_index).collect();
        let left_child: &SyntaxTreeNode = (*ast).graph.node_weight(children[1]).unwrap();

        // Save the current address for the loop
        let loop_start_addr: u8 = self.code_pointer.to_owned();

        // Starting address for the body of the while structure,
        // but 0 will never be valid, so can have default value set to 0
        let mut body_start_addr: u8 = 0x00;
        // This is the index of the body jump if a condition eveluates to false
        // that will ultimately be backpatched
        let body_jump_index: usize = self.jumps.len();

        match left_child {
            SyntaxTreeNode::NonTerminalAst(non_terminal) => {
                match &non_terminal {
                    // Evaluate the boolean expression for the while statement
                    // The Z flag is set by these function calls
                    NonTerminalsAst::IsEq => self.code_gen_compare(ast, children[1], symbol_table, true),
                    NonTerminalsAst::NotEq => self.code_gen_compare(ast, children[1], symbol_table, false),
                    _ => error!("Received {:?} when expecting IsEq or NotEq for nonterminal if expression", non_terminal)
                }
                // Add the branch code
                self.add_code(0xD0);
                self.add_jump();
                body_start_addr = self.code_pointer.to_owned();
            },
            SyntaxTreeNode::Terminal(token) => {
                match &token.token_type {
                    TokenType::Keyword(Keywords::True) => {/* Small optimization because no comparison is needed */}
                    TokenType::Keyword(Keywords::False) => {
                        // No code should be generated here because the while-statement is just dead
                        // code and will never be reached, so no point in trying to store the code
                        // with the limited space that we already have (256 bytes)
                        return;
                    }
                    _ => error!("Received {:?} when expecting true or false for while expression terminals", token)
                }
            },
            _ => error!("Received {:?} when expecting AST nonterminal or a terminal", left_child)
        }

        // Generate the code for the body
        self.code_gen_block(ast, children[0], symbol_table);

        // Get the position in the vector for the unconditional branch
        let unconditional_jump_index: usize = self.jumps.len();
        // Set X to 1
        self.add_code(0xA2);
        self.add_code(0x01);
        // 0xFF is always 0, so comparing it to 1 will result in Z = 0,
        // so the branch will always be taken
        self.add_code(0xEC);
        self.add_code(0xFF);
        self.add_code(0x00);
        self.add_code(0xD0);
        self.add_jump();

        // If there was a comparison to make, there is a start addr for the body
        // to skip over in case evaluate to false
        if body_start_addr != 0x00 {
            // Compute the difference and set it in the vector for use in backpatching
            let conditional_branch_offset: u8 = self.code_pointer - body_start_addr;
            self.jumps[body_jump_index] = conditional_branch_offset;
        }
        
        // The branch offset is the 2s complement difference between the current position
        // and the start of the loop, so take the difference and negate and add 1
        let unconditional_branch_offset: u8 = !(self.code_pointer - loop_start_addr) + 1;
        // Set the unconditional branch offset in the jump
        self.jumps[unconditional_jump_index] = unconditional_branch_offset;
    }

    fn display_code(&mut self, program_number: &u32) {
        let window: Window = web_sys::window().expect("Should be able to get the window");
        let document: Document = window.document().expect("Should be able to get the document");

        let code_gen_tabs: Element = document.get_element_by_id("code-gen-tabs").expect("Should be able to get the element");

        // Create the new tab in the list
        let new_li: Element = document.create_element("li").expect("Should be able to create the li element");

        // Add the appropriate classes
        let li_classes: DomTokenList = new_li.class_list();
        li_classes.add_1("nav-item").expect("Should be able to add the class");
        new_li.set_attribute("role", "presentation").expect("Should be able to add the attribute");

        // Create the button
        let new_button: Element = document.create_element("button").expect("Should be able to create the button");
        let btn_classes: DomTokenList = new_button.class_list();
        btn_classes.add_1("nav-link").expect("Should be able to add the class");

        // Only make the first one active
        if code_gen_tabs.child_element_count() == 0 {
            btn_classes.add_1("active").expect("Should be able to add the class");
            new_button.set_attribute("aria-selected", "true").expect("Should be able to add the attribute");
        } else {
            new_button.set_attribute("aria-selected", "false").expect("Should be able to add the attribute");
        }

        // Set the id of the button
        new_button.set_id(format!("program{}-code-gen-btn", *program_number).as_str());

        // All of the toggle elements from the example above
        new_button.set_attribute("data-bs-toggle", "tab").expect("Should be able to add the attribute");
        new_button.set_attribute("type", "button").expect("Should be able to add the attribute");
        new_button.set_attribute("role", "tab").expect("Should be able to add the attribute");
        new_button.set_attribute("data-bs-target", format!("#program{}-code-gen-pane", *program_number).as_str()).expect("Should be able to add the attribute");
        new_button.set_attribute("aria-controls", format!("program{}-code-gen-pane", *program_number).as_str()).expect("Should be able to add the attribute");

        // Set the inner text
        new_button.set_inner_html(format!("Program {}", *program_number).as_str());

        // Append the button and the list element to the area
        new_li.append_child(&new_button).expect("Should be able to add the child node");
        code_gen_tabs.append_child(&new_li).expect("Should be able to add the child node");

        // Get the content area
        let content_area: Element = document.get_element_by_id("code-gen-tab-content").expect("Should be able to find the element");

        // Create the individual pane div
        let display_area_div: Element = document.create_element("div").expect("Should be able to create the element");

        // Also from the example link above to only let the first pane initially show and be active
        let display_area_class_list: DomTokenList = display_area_div.class_list();
        display_area_class_list.add_1("tab-pane").expect("Should be able to add the class");
        if content_area.child_element_count() == 0 {
            display_area_class_list.add_2("show", "active").expect("Should be able to add the classes");
        }

        // Add the appropriate attributes
        display_area_div.set_attribute("role", "tabpanel").expect("Should be able to add the attribute");
        display_area_div.set_attribute("tabindex", "0").expect("Should be able to add the attribute");
        display_area_div.set_attribute("aria-labeledby", format!("program{}-code-gen-btn", *program_number).as_str()).expect("Should be able to add the attribute");

        // Set the id of the pane
        display_area_div.set_id(format!("program{}-code-gen-pane", *program_number).as_str());

        // The div is a container for the content of the ast info
        display_area_class_list.add_2("container", "code-gen-pane").expect("Should be able to add the classes");

        // Get the array of values but only keep the hex digits and spaces
        let mut code_str: String = format!("{:?}", self.code_arr);
        code_str.retain(|c| c != ',' && c != '[' && c != ']');

        display_area_div.set_inner_html(&code_str);

        // Add the div to the pane
        content_area.append_child(&display_area_div).expect("Should be able to add the child node");
    }

    pub fn clear_display() {
        // Get the preliminary objects
        let window: Window = web_sys::window().expect("Should be able to get the window");
        let document: Document = window.document().expect("Should be able to get the document");

        // Clear the entire area
        let tabs_area: Element = document.get_element_by_id("code-gen-tabs").expect("Should be able to find the element");
        tabs_area.set_inner_html("");
        let content_area: Element = document.get_element_by_id("code-gen-tab-content").expect("Should be able to find the element");
        content_area.set_inner_html("");
    }
}
