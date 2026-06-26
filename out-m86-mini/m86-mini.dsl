workspace "Mini & Model" {
  model {
    archimate {
      orders = applicationComponent "Orders"
      primary_node = node "Primary Node"
      deployed_artifact = artifact "Deployed Artifact"
      boundary = grouping "Boundary"
    }
    group "Users" {
      archimate {
        user_admin = businessActor "User <Admin>"
      }
    }
    user_admin -> orders "Uses" {
      type FlowRelationship
      tags "archi_view_context"
    }
    orders -> deployed_artifact "reads artifact" {
      type AccessRelationship
      access read
      tags "archi_view_context"
    }
    primary_node -> deployed_artifact "deploys" {
      type AssignmentRelationship
      tags "archi_view_context"
    }
    boundary -> orders {
      type CompositionRelationship
    }
  }
  views {
    archimateView context {
      include user_admin orders primary_node deployed_artifact
      exclude relationship.tag!=archi_view_context
      title "Context"
      properties {
        "group.Users.x" "20"
        "group.Users.y" "20"
        "group.Users.width" "220"
        "group.Users.height" "140"
        "group.Users.background" "#eeeeee"
      }
      object user_admin {
        x 40
        y 60
        width 180
        height 80
        background #ffffb5
      }
      object orders {
        x 320
        y 60
        width 180
        height 80
        background #b5ffff
      }
      object primary_node {
        x 580
        y 20
        width 180
        height 80
      }
      object deployed_artifact {
        x 580
        y 180
        width 180
        height 80
        background #eeeeff
      }
    }
  }
}
