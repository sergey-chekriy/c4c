workspace "Milestone 5 Styles" {
  model {
    user = person "User" "" "Person,Primary"
    system = softwareSystem "System" "" "SystemTag"
    user -> system "Uses" "HTTPS" "Critical"
  }
  views {
    systemLandscape styled {
      include *
    }
    styles {
      element Person {
        shape RoundedBox
        icon assets/person.svg
        width 450
        height 300
        background #112233
        color #ffffff
        stroke #445566
        strokeWidth 3
        fontSize 24
        border dashed
        opacity 90
        metadata true
        description false
        properties {
          "role" "actor"
        }
      }
      element Primary {
        background #223344
        colour #fedcba
      }
      relationship Critical {
        thickness 4
        colour #00aa00
        style dotted
        routing Orthogonal
        jump false
        fontSize 18
        width 300
        position 50
        opacity 80
        properties {
          "importance" "high"
        }
      }
      light {
        element Person {
          background #ffffff
        }
      }
      dark {
        element Person {
          background #000000
        }
      }
      theme inside-styles.json
    }
    theme local-theme.json
    themes named-theme local-two.json
    branding {
      logo assets/logo.svg
      font Inter assets/Inter.woff2
    }
    terminology {
      person Actor
      softwareSystem "Application"
      container Service
      component Module
      deploymentNode Host
      infrastructureNode Infrastructure
      relationship Interaction
      metadata round
    }
  }
}
