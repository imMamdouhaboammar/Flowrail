use anyhow::{Result, anyhow};
use flowrail_core::contract::{ContractHasher, ContractKind, ExportedSymbolContract};
use oxc_allocator::Allocator;
use oxc_ast::ast::{Declaration, Statement};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ExtractedTsSymbol {
    pub name: String,
    pub byte_range: (usize, usize),
    pub line_range: (usize, usize),
    pub is_exported: bool,
    pub is_contract: bool,
    pub mutates_state: bool,
    pub signature_hash: String,
}

pub struct TsAstSlicer;

impl TsAstSlicer {
    /// Parses a TypeScript file using oxc and extracts all top-level exported/internal declarations
    /// along with their exact byte ranges and contract hashes.
    pub fn analyze_file(file_path: &Path, source_text: &str) -> Result<Vec<ExtractedTsSymbol>> {
        let allocator = Allocator::default();
        let source_type = SourceType::from_path(file_path)
            .unwrap_or_default()
            .with_typescript(true);
        let ret = Parser::new(&allocator, source_text, source_type).parse();

        let line_starts = Self::compute_line_starts(source_text);
        let mut symbols = Vec::new();

        for stmt in &ret.program.body {
            match stmt {
                Statement::ExportNamedDeclaration(export_decl) => {
                    if let Some(decl) = &export_decl.declaration
                        && let Some(sym) = Self::extract_declaration(
                            decl,
                            export_decl.span,
                            true,
                            source_text,
                            &line_starts,
                        )
                    {
                        symbols.push(sym);
                    }
                }
                _ => {
                    if let Some(decl) = stmt.as_declaration()
                        && let Some(sym) = Self::extract_declaration(
                            decl,
                            stmt.span(),
                            false,
                            source_text,
                            &line_starts,
                        )
                    {
                        symbols.push(sym);
                    }
                }
            }
        }

        Ok(symbols)
    }

    /// Extracts all exported contracts from a TypeScript file for before/after mutation comparison.
    pub fn extract_exported_contracts(
        file_path: &Path,
        source_text: &str,
    ) -> Result<Vec<ExportedSymbolContract>> {
        let allocator = Allocator::default();
        let source_type = SourceType::from_path(file_path)
            .unwrap_or_default()
            .with_typescript(true);
        let ret = Parser::new(&allocator, source_text, source_type).parse();

        if !ret.errors.is_empty() {
            let err_msgs: Vec<String> = ret.errors.iter().map(|e| e.to_string()).collect();
            return Err(anyhow!(
                "Syntax error during AST parse: {}",
                err_msgs.join("; ")
            ));
        }

        let mut contracts = Vec::new();
        for stmt in &ret.program.body {
            if let Statement::ExportNamedDeclaration(export_decl) = stmt
                && let Some(decl) = &export_decl.declaration
            {
                match decl {
                    Declaration::TSInterfaceDeclaration(iface) => {
                        let name = iface.id.name.to_string();
                        let raw_slice =
                            &source_text[iface.span.start as usize..iface.span.end as usize];
                        let hash = blake3::hash(raw_slice.as_bytes()).to_hex().to_string();
                        contracts.push(ExportedSymbolContract {
                            symbol_name: name,
                            kind: ContractKind::Interface,
                            normalized_signature: raw_slice.to_string(),
                            signature_hash: hash,
                        });
                    }
                    Declaration::TSTypeAliasDeclaration(alias) => {
                        let name = alias.id.name.to_string();
                        let raw_slice =
                            &source_text[alias.span.start as usize..alias.span.end as usize];
                        let hash = blake3::hash(raw_slice.as_bytes()).to_hex().to_string();
                        contracts.push(ExportedSymbolContract {
                            symbol_name: name,
                            kind: ContractKind::TypeAlias,
                            normalized_signature: raw_slice.to_string(),
                            signature_hash: hash,
                        });
                    }
                    Declaration::FunctionDeclaration(func) => {
                        if let Some(id) = &func.id {
                            let name = id.name.to_string();
                            let body_start = func
                                .body
                                .as_ref()
                                .map(|b| b.span.start as usize)
                                .unwrap_or(func.span.end as usize);
                            let header_slice = &source_text[func.span.start as usize..body_start];
                            let hash = blake3::hash(header_slice.as_bytes()).to_hex().to_string();
                            contracts.push(ExportedSymbolContract {
                                symbol_name: name,
                                kind: ContractKind::FunctionSignature,
                                normalized_signature: header_slice.to_string(),
                                signature_hash: hash,
                            });
                        }
                    }
                    Declaration::ClassDeclaration(cls) => {
                        if let Some(id) = &cls.id {
                            let name = id.name.to_string();
                            let raw_slice =
                                &source_text[cls.span.start as usize..cls.span.end as usize];
                            let hash = blake3::hash(raw_slice.as_bytes()).to_hex().to_string();
                            contracts.push(ExportedSymbolContract {
                                symbol_name: name.clone(),
                                kind: ContractKind::ClassPublicSurface,
                                normalized_signature: name,
                                signature_hash: hash,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(contracts)
    }

    pub fn compute_file_contract_hash(file_path: &Path, source_text: &str) -> Result<String> {
        let contracts = Self::extract_exported_contracts(file_path, source_text)?;
        Ok(ContractHasher::hash_normalized_surface(&contracts))
    }

    pub fn slice_lines(source_text: &str, start_line: usize, end_line: usize) -> String {
        let lines: Vec<&str> = source_text.lines().collect();
        if lines.is_empty() {
            return String::new();
        }
        let s = start_line
            .saturating_sub(1)
            .min(lines.len().saturating_sub(1));
        let e = end_line.min(lines.len());
        lines[s..e]
            .iter()
            .enumerate()
            .map(|(idx, line)| format!("{:4} | {}", s + idx + 1, line))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn extract_declaration(
        decl: &Declaration,
        outer_span: Span,
        is_exported: bool,
        source_text: &str,
        line_starts: &[usize],
    ) -> Option<ExtractedTsSymbol> {
        let start = outer_span.start as usize;
        let end = outer_span.end as usize;
        let slice = &source_text[start..end.min(source_text.len())];
        let line_range = (
            Self::byte_to_line(line_starts, start),
            Self::byte_to_line(line_starts, end),
        );

        let mutates_state = slice.contains(".save(")
            || slice.contains(".insert(")
            || slice.contains(".update(")
            || slice.contains(".delete(")
            || slice.contains("prisma.")
            || slice.contains(" dispatch(");

        match decl {
            Declaration::TSInterfaceDeclaration(iface) => Some(ExtractedTsSymbol {
                name: iface.id.name.to_string(),
                byte_range: (start, end),
                line_range,
                is_exported,
                is_contract: true,
                mutates_state: false,
                signature_hash: blake3::hash(slice.as_bytes()).to_hex().to_string(),
            }),
            Declaration::TSTypeAliasDeclaration(alias) => Some(ExtractedTsSymbol {
                name: alias.id.name.to_string(),
                byte_range: (start, end),
                line_range,
                is_exported,
                is_contract: true,
                mutates_state: false,
                signature_hash: blake3::hash(slice.as_bytes()).to_hex().to_string(),
            }),
            Declaration::FunctionDeclaration(func) => {
                let name = func.id.as_ref()?.name.to_string();
                Some(ExtractedTsSymbol {
                    name,
                    byte_range: (start, end),
                    line_range,
                    is_exported,
                    is_contract: false,
                    mutates_state,
                    signature_hash: blake3::hash(slice.as_bytes()).to_hex().to_string(),
                })
            }
            Declaration::ClassDeclaration(cls) => {
                let name = cls.id.as_ref()?.name.to_string();
                Some(ExtractedTsSymbol {
                    name,
                    byte_range: (start, end),
                    line_range,
                    is_exported,
                    is_contract: false,
                    mutates_state,
                    signature_hash: blake3::hash(slice.as_bytes()).to_hex().to_string(),
                })
            }
            _ => None,
        }
    }

    fn compute_line_starts(source: &str) -> Vec<usize> {
        let mut starts = vec![0];
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                starts.push(i + 1);
            }
        }
        starts
    }

    fn byte_to_line(line_starts: &[usize], byte_offset: usize) -> usize {
        match line_starts.binary_search(&byte_offset) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        }
    }
}
