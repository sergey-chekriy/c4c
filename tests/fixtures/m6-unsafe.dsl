workspace {
  !script tools.kts
  !script {
    println disabled
  }
  !plugin com.example.Plugin
  !plugin {
    class com.example.Plugin
  }
}
