# This is a vhs script. See https://github.com/charmbracelet/vhs for more info.
# To run this script, install vhs and run `vhs ./.data/vhs.tape`
Output "target/crates-tui.mov"
Set Theme "Rose Pine"
Set Width 1400
Set Height 1200
Hide
Type "cargo run"
Enter
Sleep 2s
Show
Sleep 2s
Right @2s 3
Sleep 2s
Down @2s 3
Sleep 2s
Tab 1
Sleep 2s
Type @200ms "ratatui"
Sleep 1s
Enter
Sleep 5s
Down @2 3
Enter
Sleep 2s
Down @2 3
Enter
Sleep 2s
Up @0.5 6
Sleep 2s
Type "c"
Sleep 10s
Enter
Sleep 2s
Type "?"
Sleep 10s
