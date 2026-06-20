workspace "Milestone 7 Documentation" "Local documentation site fixture" {
  !identifiers hierarchical
  !docs m7-content/workspace
  !adrs m7-content/decisions
  model {
    bank = softwareSystem "Bank" "Banking system" {
      !docs m7-content/system.md
      !adrs m7-content/system-decisions madr
      api = container "API" "Backend API" Rust {
        !docs m7-content/api.adoc
        !adrs m7-content/api-decisions adrtools
        !adrs m7-content/log-decisions log4brains
      }
    }
  }
  views {
    systemContext bank context {
      include *
    }
    container bank containers {
      include *
    }
  }
}
