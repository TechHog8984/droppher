# Droppher

An assistant for Hypixel dropper
<br>
Droppher provides live notifications displaying information about maps.

![](assets/image1.png)
*On Windows the UI is not transparent :(
<br>
![](assets/image2.png)
<br>
![](assets/image3.png)
![](assets/image4.png)

## Todo

- [x] Notification on match start & map completion
- [x] Lunar support
- [x] Windows support
- [x] Spanish support
- [x] English support
- [ ] Badlion support
    - [x] Badlion for Windows
    - [ ] Badlion for Linux
- [ ] Map information
    - [x] Name
    - [ ] Direction to face & how to move

## Known bugs

None (yet) :D

## Download

The following platforms are supported:
<br>
- [Windows](https://github.com/TechHog8984/droppher/releases/latest/download/droppher.exe)
- [Linux](https://github.com/TechHog8984/droppher/releases/latest/download/droppher)

## Building

Windows: (this gets rid of command prompt)
```sh
$ cargo rustc --release -- -Clink-args="-Wl,--subsystem,windows"
```

Other platforms:
```sh
$ cargo build --release
```

## What things mean

If a map is in [the list](assets/map_information.json), you will see information about that map in its notification.

1. The first piece of information is <ins>difficulty</ins> (<span style="color:#80ed99">easy</span>, <span style="color:#fcbf49">medium</span>, or <span style="color:#ef233c">hard</span>).

2. The second piece of information is either a <ins>tip on how to complete the map</ins> or "no tip". This usually includes a direction to face.

3. The third piece of information is whether a map has a <ins>[portal skip](#portal-skip)</ins>. This will be empty if there is no known portal skip.

4. The fourth piece of information is <ins>a tip for the portal skip</ins>. This can also be empty and is similar to the other tip.

## Portal skip

Maps with portal skips are maps that allow the player to ignore the water and head straight into the portal. This usually is difficult, especially on maps like <ins>Vintage</ins>, but it can be easy and it depends entirely on the map.

## Contributing

If you want to contribute [code](src/main.rs) or [map information](assets/map_information.json), submit a pull request!
<br>
If you want to contribute language support or if you need help with pull requests, reach out to me on discord @techhog

## Bugs

Find a bug? [Report an issue](https://github.com/TechHog8984/droppher/issues/new/choose).