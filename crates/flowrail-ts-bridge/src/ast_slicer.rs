use anyhow::{Result, anyhow};
use flowrail_core::contract::{ContractHasher, ContractKind, ExportedSymbolContract};
use oxc_allocator::Allocator;
use oxc_ast::ast::{Declaration, ImportDeclarationSpecifier, ImportOrExportKind, Statement};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedImport {
    pub source: String,
    pub specifiers: Vec<ImportSpecifierDetail>,
    pub is_type_only: bool,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSpecifierDetail {
    pub imported_name: String,
    pub local_name: String,
    pub is_type_only: bool,
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
            collect_statement_contracts(stmt, source_text, &mut contracts);
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

    /// Extracts all import declarations (and re-export sources) from a TypeScript file using oxc.
    pub fn extract_imports(file_path: &Path, source_text: &str) -> Result<Vec<ExtractedImport>> {
        let allocator = Allocator::default();
        let source_type = SourceType::from_path(file_path)
            .unwrap_or_default()
            .with_typescript(true);
        let ret = Parser::new(&allocator, source_text, source_type).parse();
        let line_starts = Self::compute_line_starts(source_text);
        let mut imports = Vec::new();

        for stmt in &ret.program.body {
            match stmt {
                Statement::ImportDeclaration(import_decl) => {
                    let source = import_decl.source.value.to_string();
                    let line = Self::byte_to_line(&line_starts, import_decl.span.start as usize);
                    let is_type_only = import_decl.import_kind == ImportOrExportKind::Type;
                    let mut specifiers = Vec::new();

                    if let Some(specs) = &import_decl.specifiers {
                        for spec in specs {
                            match spec {
                                ImportDeclarationSpecifier::ImportSpecifier(s) => {
                                    let imported_name = s.imported.name().to_string();
                                    let local_name = s.local.name.to_string();
                                    let spec_type_only =
                                        is_type_only || s.import_kind == ImportOrExportKind::Type;
                                    specifiers.push(ImportSpecifierDetail {
                                        imported_name,
                                        local_name,
                                        is_type_only: spec_type_only,
                                    });
                                }
                                ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                                    specifiers.push(ImportSpecifierDetail {
                                        imported_name: "default".to_string(),
                                        local_name: s.local.name.to_string(),
                                        is_type_only,
                                    });
                                }
                                ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                                    specifiers.push(ImportSpecifierDetail {
                                        imported_name: "*".to_string(),
                                        local_name: s.local.name.to_string(),
                                        is_type_only,
                                    });
                                }
                            }
                        }
                    }

                    imports.push(ExtractedImport {
                        source,
                        specifiers,
                        is_type_only,
                        line,
                    });
                }
                Statement::ExportNamedDeclaration(export_decl) => {
                    if let Some(src) = &export_decl.source {
                        let source = src.value.to_string();
                        let line =
                            Self::byte_to_line(&line_starts, export_decl.span.start as usize);
                        let is_type_only = export_decl.export_kind == ImportOrExportKind::Type;
                        let mut specifiers = Vec::new();

                        for spec in &export_decl.specifiers {
                            let imported_name = spec.local.name().to_string();
                            let local_name = spec.exported.name().to_string();
                            let spec_type_only =
                                is_type_only || spec.export_kind == ImportOrExportKind::Type;
                            specifiers.push(ImportSpecifierDetail {
                                imported_name,
                                local_name,
                                is_type_only: spec_type_only,
                            });
                        }

                        imports.push(ExtractedImport {
                            source,
                            specifiers,
                            is_type_only,
                            line,
                        });
                    }
                }
                Statement::ExportAllDeclaration(export_all) => {
                    let source = export_all.source.value.to_string();
                    let line = Self::byte_to_line(&line_starts, export_all.span.start as usize);
                    let is_type_only = export_all.export_kind == ImportOrExportKind::Type;
                    imports.push(ExtractedImport {
                        source,
                        specifiers: vec![ImportSpecifierDetail {
                            imported_name: "*".to_string(),
                            local_name: "*".to_string(),
                            is_type_only,
                        }],
                        is_type_only,
                        line,
                    });
                }
                _ => {}
            }
        }

        Ok(imports)
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

fn collect_statement_contracts(
    stmt: &Statement,
    source_text: &str,
    contracts: &mut Vec<ExportedSymbolContract>,
) {
    let Statement::ExportNamedDeclaration(export_decl) = stmt else {
        return;
    };
    let Some(decl) = &export_decl.declaration else {
        return;
    };
    match decl {
        Declaration::TSInterfaceDeclaration(iface) => {
            contracts.push(extract_interface_contract(iface, source_text));
        }
        Declaration::TSTypeAliasDeclaration(alias) => {
            contracts.push(extract_type_alias_contract(alias, source_text));
        }
        Declaration::FunctionDeclaration(func) => {
            contracts.extend(extract_function_contract(func, source_text));
        }
        Declaration::ClassDeclaration(cls) => {
            contracts.extend(extract_class_contract(cls, source_text));
        }
        Declaration::VariableDeclaration(var_decl) => {
            extract_variable_contracts(var_decl, source_text, contracts);
        }
        _ => {}
    }
}

fn extract_interface_contract(
    iface: &oxc_ast::ast::TSInterfaceDeclaration,
    source_text: &str,
) -> ExportedSymbolContract {
    let name = iface.id.name.to_string();
    let raw_slice = &source_text[iface.span.start as usize..iface.span.end as usize];
    let hash = blake3::hash(raw_slice.as_bytes()).to_hex().to_string();
    ExportedSymbolContract {
        symbol_name: name,
        kind: ContractKind::Interface,
        normalized_signature: raw_slice.to_string(),
        signature_hash: hash,
    }
}

fn extract_type_alias_contract(
    alias: &oxc_ast::ast::TSTypeAliasDeclaration,
    source_text: &str,
) -> ExportedSymbolContract {
    let name = alias.id.name.to_string();
    let raw_slice = &source_text[alias.span.start as usize..alias.span.end as usize];
    let hash = blake3::hash(raw_slice.as_bytes()).to_hex().to_string();
    ExportedSymbolContract {
        symbol_name: name,
        kind: ContractKind::TypeAlias,
        normalized_signature: raw_slice.to_string(),
        signature_hash: hash,
    }
}

fn extract_function_contract(
    func: &oxc_ast::ast::Function,
    source_text: &str,
) -> Option<ExportedSymbolContract> {
    let id = func.id.as_ref()?;
    let name = id.name.to_string();
    let body_start = func
        .body
        .as_ref()
        .map_or(func.span.end as usize, |b| b.span.start as usize);
    let header_slice = &source_text[func.span.start as usize..body_start];
    let hash = blake3::hash(header_slice.as_bytes()).to_hex().to_string();
    Some(ExportedSymbolContract {
        symbol_name: name,
        kind: ContractKind::FunctionSignature,
        normalized_signature: header_slice.to_string(),
        signature_hash: hash,
    })
}

fn extract_variable_contracts(
    var_decl: &oxc_ast::ast::VariableDeclaration,
    source_text: &str,
    contracts: &mut Vec<ExportedSymbolContract>,
) {
    for declarator in &var_decl.declarations {
        let oxc_ast::ast::BindingPatternKind::BindingIdentifier(ident) = &declarator.id.kind else {
            continue;
        };
        let name = ident.name.to_string();
        let raw_slice = &source_text[declarator.span.start as usize..declarator.span.end as usize];
        let hash = blake3::hash(raw_slice.as_bytes()).to_hex().to_string();
        contracts.push(ExportedSymbolContract {
            symbol_name: name,
            kind: ContractKind::FunctionSignature,
            normalized_signature: raw_slice.to_string(),
            signature_hash: hash,
        });
    }
}

fn extract_class_contract(
    cls: &oxc_ast::ast::Class,
    source_text: &str,
) -> Option<ExportedSymbolContract> {
    let id = cls.id.as_ref()?;
    let name = id.name.to_string();
    let raw_slice = &source_text[cls.span.start as usize..cls.span.end as usize];
    let hash = blake3::hash(raw_slice.as_bytes()).to_hex().to_string();
    let class_surface = extract_class_public_surface(cls, &name, source_text);

    Some(ExportedSymbolContract {
        symbol_name: name,
        kind: ContractKind::ClassPublicSurface,
        normalized_signature: class_surface,
        signature_hash: hash,
    })
}

fn extract_class_public_surface(
    cls: &oxc_ast::ast::Class,
    class_name: &str,
    source_text: &str,
) -> String {
    let mut member_sigs = Vec::new();
    for elem in &cls.body.body {
        if let Some(sig) = extract_class_element_signature(elem, source_text) {
            member_sigs.push(sig);
        }
    }
    format!("class {} {{ {} }}", class_name, member_sigs.join("; "))
}

fn extract_class_element_signature(
    elem: &oxc_ast::ast::ClassElement,
    source_text: &str,
) -> Option<String> {
    match elem {
        oxc_ast::ast::ClassElement::MethodDefinition(method) => {
            if method.accessibility == Some(oxc_ast::ast::TSAccessibility::Private) {
                return None;
            }
            let body_start = method
                .value
                .body
                .as_ref()
                .map(|b| b.span.start as usize)
                .unwrap_or(method.span.end as usize);
            let sig_part = &source_text[method.span.start as usize..body_start];
            Some(sig_part.trim().to_string())
        }
        oxc_ast::ast::ClassElement::PropertyDefinition(prop) => {
            if prop.accessibility == Some(oxc_ast::ast::TSAccessibility::Private) {
                return None;
            }
            let prop_sig = &source_text[prop.span.start as usize..prop.span.end as usize];
            Some(prop_sig.trim().to_string())
        }
        oxc_ast::ast::ClassElement::TSIndexSignature(idx_sig) => {
            let sig = &source_text[idx_sig.span.start as usize..idx_sig.span.end as usize];
            Some(sig.trim().to_string())
        }
        _ => None,
    }
}
