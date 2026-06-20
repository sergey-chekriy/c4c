workspace "Milestone 5 Remote References" {
  model {
    user = person User "" Person
  }
  views {
    systemLandscape remote {
      include *
    }
    styles {
      element Person {
        icon "https://example.test/person.svg"
      }
    }
    theme "https://example.test/theme.json"
    themes local-theme.json "https://example.test/second-theme.json"
    branding {
      logo "https://example.test/logo.svg"
      font Inter "https://example.test/font.woff2"
    }
  }
}
