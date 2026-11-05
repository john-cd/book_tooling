//! Conversion from `fmt::Write` to `io::Write`.
//!
//! Provides utilities to adapt `fmt::Write` interfaces to `io::Write` streams.
//!
//! TODO Reviewed ??-??-2026

use std::io;
use std::io::BufWriter;

use fmt2io::Writer;

use super::Visitable as _;
use super::Visitor;
use crate::ast::Document;

impl<'d> Document<'d> {
    /// Adapt from `fmt::Write` to `io::Write`.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if writing to `io_writer` fails.
    pub fn to_io<'v, IO, F, V>(&'d self, mut io_writer: IO, f: F) -> io::Result<()>
    where
        IO: io::Write,
        V: Visitor<'v>,
        F: FnOnce(&mut Writer<&mut IO>) -> V,
        'd: 'v,
    {
        // The `write` function takes a `io::Write`, converts it to a struct that implements
        // `fmt::Write` and provides it to the closure.
        fmt2io::write(&mut io_writer, |writer| {
            let mut visitor = f(writer);
            self.accept(&mut visitor);
            Ok(())
        })?;
        // Surface any I/O errors:
        io_writer.flush()?;
        Ok(())
    }

    /// Create a buffer writer from an IO writer,
    /// convert to `fmt::Write`, pass to a closure that returns a visitor,
    /// accept the visitor.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if writing to `io_writer` fails.
    pub fn with_buffer<'v, IO, F, V>(
        &'d self,
        capacity: usize,
        inner_io_writer: IO,
        f: F,
    ) -> io::Result<()>
    where
        IO: io::Write,
        V: Visitor<'v>,
        F: FnOnce(&mut Writer<&mut BufWriter<IO>>) -> V,
        'd: 'v,
    {
        use io::BufWriter;
        use io::Write as _;
        let mut io_writer = BufWriter::with_capacity(capacity, inner_io_writer);
        fmt2io::write(&mut io_writer, |writer| {
            let mut visitor = f(writer);
            self.accept(&mut visitor);
            Ok(())
        })?;
        io_writer.flush()?;
        Ok(())
    }

    /// Adapt from `fmt::Write` to `stdout`.
    /// Accept the visitor returned by the closure.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if writing to standard output fails.
    pub fn to_stdout<'v, F, V>(&'d self, f: F) -> io::Result<()>
    where
        'd: 'v,
        V: Visitor<'v>,
        F: FnOnce(&mut Writer<&mut io::Stdout>) -> V,
    {
        let io_writer = io::stdout();
        self.to_io(io_writer, f)
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;
    use crate::parse_document;

    /// Visitor that ignores every element; the tests write through the adapted writer directly.
    struct Noop;
    impl Visitor<'_> for Noop {}

    struct FailingWriter;
    impl io::Write for FailingWriter {
        /// Verifies: write.
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("disk full"))
        }

        /// Verifies: flush.
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("disk full"))
        }
    }

    /// Verifies: to io forwards fmt writes to the io writer.
    #[test]
    fn to_io_forwards_fmt_writes_to_the_io_writer() {
        let doc = parse_document(
            "t", None, "# Title
",
        )
        .expect("parses");
        let mut out = Vec::new();
        doc.to_io(&mut out, |writer| {
            write!(writer, "hello").expect("fmt write");
            Noop
        })
        .expect("writes");
        assert_eq!(out, b"hello");
    }

    /// Verifies: with buffer flushes to the inner writer.
    #[test]
    fn with_buffer_flushes_to_the_inner_writer() {
        let doc = parse_document(
            "t", None, "x
",
        )
        .expect("parses");
        let mut out = Vec::new();
        doc.with_buffer(1024, &mut out, |writer| {
            write!(writer, "buffered").expect("fmt write");
            Noop
        })
        .expect("writes");
        assert_eq!(out, b"buffered");
    }

    /// Verifies: write failures are propagated.
    #[test]
    fn write_failures_are_propagated() {
        let doc = parse_document(
            "t", None, "x
",
        )
        .expect("parses");
        let err = doc
            .to_io(FailingWriter, |writer| {
                let _unused = write!(writer, "data");
                Noop
            })
            .expect_err("failure surfaces");
        assert_eq!(err.to_string(), "disk full");
    }

    /// Verifies: flush failure is propagated when buffered writes succeed.
    #[test]
    fn flush_failure_is_propagated_when_buffered_writes_succeed() {
        let doc = parse_document(
            "t", None, "x
",
        )
        .expect("parses");
        doc.with_buffer(1024, FailingWriter, |writer| {
            let _unused = write!(writer, "small");
            Noop
        })
        .expect_err("flush error surfaces");
    }
}
