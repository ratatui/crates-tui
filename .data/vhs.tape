# This is a vhs script. See https://github.com/charmbracelet/vhs for more info.
# To run this script, install vhs and run `vhs ./.data/vhs.tape`
Output "target/crates-tui.mov"
Set Theme "Rose Pine"
Set Width 1200
Set Height 1400
Hide
Type "cargo run"
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
Sleep 2s
Down @2 3
Enter
Down @2 3
Enter
Up @2 6
