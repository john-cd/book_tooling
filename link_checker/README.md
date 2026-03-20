# link_checker

CLI tool for verifying external links in Markdown files.

## Features

- **Async Validation**: Checks multiple links concurrently for better performance.
- **Caching**: Uses `moka` to cache results and avoid redundant network requests.
