// TODO Reviewed ??-??-2026
#[cfg(test)]
mod template_tests {
    use crate::templates::DestinationKind;
    use crate::templates::render_badge;
    use crate::templates::render_link;

    #[test]
    fn test_render_badge_crate() {
        let name = "clap";
        let expected = "[![clap][p~clap~crate~badge]][p~clap~crate]{{hi:clap}}";
        assert_eq!(render_badge(DestinationKind::Crate, name), expected);
    }

    #[test]
    fn test_render_badge_docs() {
        let name = "clap";
        let expected = "[![clap][c~clap~docs~badge]][c~clap~docs]{{hi:clap}}";
        assert_eq!(render_badge(DestinationKind::Docs, name), expected);
    }

    #[test]
    fn test_render_link_category() {
        let name = "accessibility";
        let expected = "[accessibility][cat~accessibility]\u{2197}{{hi:accessibility}}";
        assert_eq!(render_link(DestinationKind::Category, name), expected);
    }
}

/// Tests shared expected outputs for the `rust_rate` template functions.
#[cfg(test)]
mod shared_template_fixtures {
    use std::io;

    use crate::templates::DestinationKind;
    use crate::templates::render_badge;
    use crate::templates::render_link;

    /// Verifies `rust_rate` renders the same destinations as the shared `book_tool` fixture contract.
    #[test]
    fn template_outputs_match_shared_destination_fixtures() -> Result<(), Box<dyn std::error::Error>>
    {
        for line in include_str!("../../../test_fixtures/template_output.tsv").lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let mut fields = line.split('\t');
            let kind_name = fields
                .next()
                .ok_or_else(|| io::Error::other("fixture is missing a destination kind"))?;
            let kind = match kind_name {
                "category" => DestinationKind::Category,
                "crate" => DestinationKind::Crate,
                "docs" => DestinationKind::Docs,
                "github-repo" => DestinationKind::GithubRepo,
                "lib-rs" => DestinationKind::LibRs,
                "crates-io" => DestinationKind::CratesIo,
                "web" => DestinationKind::Web,
                other => {
                    return Err(
                        io::Error::other(format!("unknown destination kind: {other}")).into(),
                    );
                }
            };
            let name = fields
                .next()
                .ok_or_else(|| io::Error::other("fixture is missing a destination name"))?;
            let expected_badge = fields
                .next()
                .ok_or_else(|| io::Error::other("fixture is missing a badge expectation"))?;
            let expected_link = fields
                .next()
                .ok_or_else(|| io::Error::other("fixture is missing a link expectation"))?;
            if fields.next().is_some() {
                return Err(io::Error::other("fixture contains extra fields").into());
            }

            assert_eq!(
                render_badge(kind, name),
                expected_badge,
                "kind: {kind_name}"
            );
            assert_eq!(render_link(kind, name), expected_link, "kind: {kind_name}");
        }
        Ok(())
    }
}
