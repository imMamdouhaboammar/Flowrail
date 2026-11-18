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
