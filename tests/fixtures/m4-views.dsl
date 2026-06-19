workspace "Milestone 4 Views" {
  !identifiers hierarchical
  model {
    user = person "User" "" "External"
    system = softwareSystem "System" "" "Core" {
      web = container "Web" "" "Rust" "Web" {
        controller = component "Controller" "" "Rust" "Internal"
      }
      api = container "API" "" "Rust" "Internal"
    }
    external = softwareSystem "External" "" "External"
    note = element "Note" "Annotation"
    user -> system.web "Uses" "HTTPS" "Critical"
    system.web.controller -> system.api "Calls"
    system.api -> external "Sends" "HTTPS" "External"
  }
  views {
    systemLandscape landscape "All systems" {
      include *
      autoLayout tb 100 200
      default
      animation {
        user
        system external
      }
      title "Landscape title"
      description "Landscape description"
      properties {
        "owner" "Architecture"
      }
    }
    systemLandscape selected {
      include user system external
      exclude external
      autoLayout bt
    }
    systemContext system context {
      include *
      include element.tag==Core
      exclude "external -> *"
      autoLayout lr
    }
    container system containers {
      include *?
      autoLayout rl
    }
    component system.web components {
      include *
      exclude system.api
    }
    filtered context include Core filteredInclude "Core only" {
    }
    filtered context exclude External filteredExclude "No external" {
    }
    dynamic system flow "Request flow" {
      2: user -> system.web "Open page" "HTTPS"
      1: system.api -> external "Fetch data" "HTTPS"
    }
    custom customDiagram "Custom title" "Custom description" {
      include note
    }
    image * localImages {
      plantuml diagrams/system.puml
      mermaid diagrams/system.mmd
      kroki d2 diagrams/system.d2
      image diagrams/system.png
    }
    properties {
      "viewset" "M4"
    }
    styles {
      element Person {
        shape Person
      }
    }
    theme local-theme.json
    themes local-one.json local-two.json
    terminology {
      person Actor
    }
  }
}
