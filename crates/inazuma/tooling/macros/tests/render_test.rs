#[test]
fn test_derive_render() {
    use inazuma_macros::Render;

    #[derive(Render)]
    struct _Element;
}
