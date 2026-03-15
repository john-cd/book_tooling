// [test new templating](https://github.com/john-cd/rust_howto/issues/1431)

#[cfg(test)]
mod template_tests {
    use crate::templates::DestinationKind;
    use crate::templates::render_badge;
    use crate::templates::render_link;

    #[test]
    fn test_render_badge_crate() {
        let name = "clap";
        let expected = "[![clap][p~clap~crate~badge]][p~clap~crate]{{hi:clap}}";
        assert_eq!(render_badge(&DestinationKind::Crate, name), expected);
    }

    #[test]
    fn test_render_badge_docs() {
        let name = "clap";
        let expected = "[![clap][c~clap~docs~badge]][c~clap~docs]{{hi:clap}}";
        assert_eq!(render_badge(&DestinationKind::Docs, name), expected);
    }

    #[test]
    fn test_render_link_category() {
        let name = "accessibility";
        let expected = "[accessibility][cat~accessibility]↗{{hi:accessibility}}";
        assert_eq!(render_link(&DestinationKind::Category, name), expected);
    }
}
