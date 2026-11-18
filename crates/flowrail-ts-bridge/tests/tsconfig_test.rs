use flowrail_ts_bridge::scip_loader::ScipGraphBuilder;
use flowrail_ts_bridge::tsconfig::TsConfigResolver;
use std::fs;
use std::path::Path;

#[test]
fn test_strip_jsonc_comments() {
    let jsonc = r#"{
        // This is a comment
        "compilerOptions": {
            /* multi-line
               comment */
            "baseUrl": ".",
            "url": "https://example.com/api" // comment after string
        }
    }"#;

    let clean = TsConfigResolver::strip_jsonc_comments(jsonc);
    let parsed: serde_json::Value =
        serde_json::from_str(&clean).expect("Should parse stripped JSON");
    assert_eq!(parsed["compilerOptions"]["baseUrl"].as_str(), Some("."));
    assert_eq!(
        parsed["compilerOptions"]["url"].as_str(),
        Some("https://example.com/api")
    );
}

#[test]
fn test_tsconfig_path_alias_resolution() {
    let tmp_dir =
        std::env::temp_dir().join(format!("flowrail_test_tsconfig_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(tmp_dir.join("src/modules/checkout")).unwrap();
    fs::create_dir_all(tmp_dir.join("src/modules/payment")).unwrap();

    let tsconfig_content = r#"{
        "compilerOptions": {
            "baseUrl": ".",
            "paths": {
                "@/*": ["src/*"],
                "@payment/*": ["src/modules/payment/*"]
            }
        }
    }"#;
    fs::write(tmp_dir.join("tsconfig.json"), tsconfig_content).unwrap();

    let checkout_controller = r#"
        import { PaymentService } from '@payment/payment.service';
        import { OrderModel } from '@/modules/checkout/order.model';

        export class CheckoutController {
            constructor(private paymentService: PaymentService) {}
            process(order: OrderModel) {
                this.paymentService.charge();
            }
        }
    "#;
    fs::write(
        tmp_dir.join("src/modules/checkout/checkout.controller.ts"),
        checkout_controller,
    )
    .unwrap();

    let payment_service = r#"
        export class PaymentService {
            charge() {
                // payment logic
            }
        }
    "#;
    fs::write(
        tmp_dir.join("src/modules/payment/payment.service.ts"),
        payment_service,
    )
    .unwrap();

    let order_model = r#"
        export interface OrderModel {
            id: string;
            amount: number;
        }
    "#;
    fs::write(
        tmp_dir.join("src/modules/checkout/order.model.ts"),
        order_model,
    )
    .unwrap();

    let resolver = TsConfigResolver::load_from_workspace(&tmp_dir).expect("Should load tsconfig");
    let importing_file = Path::new("src/modules/checkout/checkout.controller.ts");

    // Test resolving @payment/payment.service
    let resolved_payment = resolver
        .resolve_module(&tmp_dir, importing_file, "@payment/payment.service")
        .expect("Should resolve @payment path alias");
    assert_eq!(
        resolved_payment,
        Path::new("src/modules/payment/payment.service.ts")
    );

    // Test resolving @/modules/checkout/order.model
    let resolved_order = resolver
        .resolve_module(&tmp_dir, importing_file, "@/modules/checkout/order.model")
        .expect("Should resolve @/* path alias");
    assert_eq!(
        resolved_order,
        Path::new("src/modules/checkout/order.model.ts")
    );

    // Test building graph with path aliases
    let graph = ScipGraphBuilder::build_from_directory(&tmp_dir).expect("Should build graph");
    assert!(graph.node_count() >= 3);
    assert!(graph.edge_count() >= 2);

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_clean_specifier_protocols_and_aliases() {
    assert_eq!(
        TsConfigResolver::clean_specifier("npm:@ecommerce/core@^1.0.0"),
        "@ecommerce/core"
    );
    assert_eq!(
        TsConfigResolver::clean_specifier("npm:@ecommerce/core@1.0.0/tax"),
        "@ecommerce/core/tax"
    );
    assert_eq!(
        TsConfigResolver::clean_specifier("npm:lodash@4.17.21"),
        "lodash"
    );
    assert_eq!(
        TsConfigResolver::clean_specifier("npm:date-fns@2.0.0/format"),
        "date-fns/format"
    );
    assert_eq!(
        TsConfigResolver::clean_specifier("workspace:@scope/pkg"),
        "@scope/pkg"
    );
    assert_eq!(
        TsConfigResolver::clean_specifier("./checkout.service"),
        "./checkout.service"
    );
}

#[test]
fn test_monorepo_multi_package_workspace_resolution() {
    let tmp_dir =
        std::env::temp_dir().join(format!("flowrail_test_monorepo_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp_dir);

    // Monorepo layout:
    // packages/core
    // packages/tax
    // apps/web
    fs::create_dir_all(tmp_dir.join("packages/core/src/models")).unwrap();
    fs::create_dir_all(tmp_dir.join("packages/tax/src")).unwrap();
    fs::create_dir_all(tmp_dir.join("apps/web/src")).unwrap();

    // 1. packages/core/package.json
    let core_pkg_json = r#"{
        "name": "@ecommerce/core",
        "exports": {
            ".": "./src/index.ts",
            "./models": "./src/models/order.ts"
        }
    }"#;
    fs::write(tmp_dir.join("packages/core/package.json"), core_pkg_json).unwrap();

    let core_index = r#"
        export class CoreService {
            init() { return true; }
        }
    "#;
    fs::write(tmp_dir.join("packages/core/src/index.ts"), core_index).unwrap();

    let core_order_model = r#"
        export interface OrderModel {
            id: string;
            amount: number;
        }
    "#;
    fs::write(
        tmp_dir.join("packages/core/src/models/order.ts"),
        core_order_model,
    )
    .unwrap();

    // 2. packages/tax/package.json
    let tax_pkg_json = r#"{
        "name": "@ecommerce/tax",
        "main": "./src/calculator.ts"
    }"#;
    fs::write(tmp_dir.join("packages/tax/package.json"), tax_pkg_json).unwrap();

    let tax_calculator = r#"
        export class TaxCalculator {
            calculate(amount: number): number {
                return amount * 0.2;
            }
        }
    "#;
    fs::write(
        tmp_dir.join("packages/tax/src/calculator.ts"),
        tax_calculator,
    )
    .unwrap();

    // 3. apps/web/src/checkout.ts importing packages via package names and npm: alias
    let web_checkout = r#"
        import { CoreService } from '@ecommerce/core';
        import { OrderModel } from '@ecommerce/core/models';
        import { TaxCalculator } from 'npm:@ecommerce/tax@^2.0.0';

        export class CheckoutWorkflow {
            constructor(
                private core: CoreService,
                private tax: TaxCalculator
            ) {}

            process(order: OrderModel): number {
                this.core.init();
                return this.tax.calculate(order.amount);
            }
        }
    "#;
    fs::write(tmp_dir.join("apps/web/src/checkout.ts"), web_checkout).unwrap();

    let resolver =
        TsConfigResolver::load_from_workspace(&tmp_dir).expect("Should discover monorepo packages");
    assert!(resolver.packages.contains_key("@ecommerce/core"));
    assert!(resolver.packages.contains_key("@ecommerce/tax"));

    let importing_file = Path::new("apps/web/src/checkout.ts");

    // Test resolving root package: @ecommerce/core -> packages/core/src/index.ts
    let res_core = resolver
        .resolve_module(&tmp_dir, importing_file, "@ecommerce/core")
        .expect("Should resolve @ecommerce/core root package");
    assert_eq!(res_core, Path::new("packages/core/src/index.ts"));

    // Test resolving subpath export: @ecommerce/core/models -> packages/core/src/models/order.ts
    let res_models = resolver
        .resolve_module(&tmp_dir, importing_file, "@ecommerce/core/models")
        .expect("Should resolve @ecommerce/core/models subpath export");
    assert_eq!(res_models, Path::new("packages/core/src/models/order.ts"));

    // Test resolving npm: protocol alias: npm:@ecommerce/tax@^2.0.0 -> packages/tax/src/calculator.ts
    let res_tax = resolver
        .resolve_module(&tmp_dir, importing_file, "npm:@ecommerce/tax@^2.0.0")
        .expect("Should resolve npm: protocol alias for @ecommerce/tax");
    assert_eq!(res_tax, Path::new("packages/tax/src/calculator.ts"));

    // Test end-to-end graph construction over monorepo packages
    let graph =
        ScipGraphBuilder::build_from_directory(&tmp_dir).expect("Should build monorepo graph");
    assert!(graph.node_count() >= 4);
    assert!(graph.edge_count() >= 3);

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_root_relative_specifier_resolution() {
    let tmp_dir =
        std::env::temp_dir().join(format!("flowrail_test_root_rel_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(tmp_dir.join("src/components")).unwrap();
    fs::create_dir_all(tmp_dir.join("src/views")).unwrap();

    fs::write(
        tmp_dir.join("src/components/button.tsx"),
        "export const Button = () => null;",
    )
    .unwrap();

    let resolver = TsConfigResolver::default();
    let importing_file = Path::new("src/views/home.tsx");

    // Root-relative import "/src/components/button" must resolve inside tmp_dir, NOT at host /src/components/button
    let resolved = resolver
        .resolve_module(&tmp_dir, importing_file, "/src/components/button")
        .expect("Root-relative import should resolve inside workspace");
    assert_eq!(resolved, Path::new("src/components/button.tsx"));

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_package_prefix_longest_match_priority() {
    let tmp_dir =
        std::env::temp_dir().join(format!("flowrail_test_pkg_priority_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp_dir);

    // Packages: @org/core and @org/core-utils
    fs::create_dir_all(tmp_dir.join("packages/core/src")).unwrap();
    fs::create_dir_all(tmp_dir.join("packages/core-utils/src")).unwrap();

    let core_pkg = r#"{"name": "@org/core", "main": "./src/index.ts"}"#;
    fs::write(tmp_dir.join("packages/core/package.json"), core_pkg).unwrap();
    fs::write(tmp_dir.join("packages/core/src/index.ts"), "export const core = 1;").unwrap();

    let utils_pkg = r#"{"name": "@org/core-utils", "main": "./src/index.ts"}"#;
    fs::write(tmp_dir.join("packages/core-utils/package.json"), utils_pkg).unwrap();
    fs::write(
        tmp_dir.join("packages/core-utils/src/index.ts"),
        "export const utils = 2;",
    )
    .unwrap();

    let resolver = TsConfigResolver::load_from_workspace(&tmp_dir).unwrap();
    let importing_file = Path::new("apps/web/index.ts");

    // Subpath or exact import to @org/core-utils must match @org/core-utils, not @org/core
    let res = resolver
        .resolve_module(&tmp_dir, importing_file, "@org/core-utils")
        .expect("Should resolve @org/core-utils");
    assert_eq!(res, Path::new("packages/core-utils/src/index.ts"));

    let _ = fs::remove_dir_all(&tmp_dir);
}
