workspace "M84 Invalid ArchiMate" {
  model {
    archimate {
      objective = goal "Unsafe objective"
      runtime = node "Runtime"
      objective -> runtime "questionable flow" {
        type FlowRelationship
      }
      runtime -> objective "invalid access" {
        type AccessRelationship
        access sideways
      }
    }
  }

  views {
    archimateView invalid {
      viewpoint nonsense
      include *
    }
  }
}
