workspace "M8.3 ArchiMate Profile" {
  model {
    archimate {
      operator = businessActor "Operator"

      gateway = applicationComponent "Internal API Gateway" {
        background #008e00
        color #ffffff
        stroke #006600
        fontSize 12
        width 180
        height 80
      }

      ledger = applicationComponent "External Ledger"
      cache = node "Cache Engine"

      gateway -> ledger "Posts entries" {
        type FlowRelationship
        color #00aa00
        thickness 2
        style dashed
      }

      gateway -> cache "Reads cached data" {
        type Access
      }
    }
  }

  views {
    archimateView dex-native {
      include *

      object gateway {
        x 300
        y 120
        width 180
        height 80
        background #008e00
      }

      object ledger {
        x 600
        y 120
        width 180
        height 80
      }
    }
  }
}
