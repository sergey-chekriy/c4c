workspace "Milestone 5 Invalid Styles" {
  model {
    user = person User "" Person
    system = softwareSystem System
    user -> system Uses "" Critical
  }
  views {
    systemLandscape invalid {
      include *
    }
    styles {
      element Person {
        shape Blob
        width wide
        strokeWidth 11
        border double
        opacity 101
        metadata perhaps
        description perhaps
      }
      relationship Critical {
        thickness thick
        style broken
        routing ZigZag
        jump perhaps
        position -1
        opacity 101
      }
    }
    terminology {
      metadata triangle
    }
  }
}
