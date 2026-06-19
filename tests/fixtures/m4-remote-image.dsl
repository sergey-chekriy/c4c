workspace "Remote images are unsafe" {
  views {
    image * remote {
      plantuml "https://example.test/diagram.puml"
      mermaid "https://example.test/diagram.mmd"
      kroki d2 "https://example.test/diagram.d2"
      image "https://example.test/diagram.png"
    }
  }
}
