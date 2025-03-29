use crate::macros::mikrotik_model;
use proc_macro2::TokenStream;
use syn::parse_quote;

#[test]
fn test_macro() {
    let attr: TokenStream = parse_quote! {
        name=DeviceData,
        detect=new,
        fields(
            systemIdentity(single="system/identity"),
            ethernet(by_key(path = "interface/ethernet", key = defaultName)),
            bridge(by_key(path="interface/bridge",key=name)),
            bridge_port(by_id(path="interface/bridge/port",keys(bridge,interface)))
        )
    };
    let result = mikrotik_model(attr).expect("failed to run model");
    let file = syn::parse2(result).unwrap();
    println!("{}", prettyplease::unparse(&file));
}

#[test]
fn test_routerboard() {
    let attr: TokenStream = parse_quote! {
        name=DeviceData,
        detect=new,
        fields(
            routerboard(single= "system/routerboard"),
        )
    };
    let result = mikrotik_model(attr).expect("failed to run model");
    let file = syn::parse2(result).unwrap();
    println!("{}", prettyplease::unparse(&file));
}
