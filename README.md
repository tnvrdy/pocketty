# pocketty - produce music in your terminal

## inspiration
we love music production and good design, teenage engineering does these two best. TE's Pocket Operator is pretty tiny but allows basically anyone to produce music worth listening to. intentionally, given its size, the device itself is constrained in terms of power, memory, production event space, and preservation of your work. we thought having this music production system on your computer, while still keeping it ~quaint~, would be useful and fun. and tuis are cute! voila, a pocket operator tui named pocke*tty*.

## what it does
implements all music production functionality of the pocket operator, including sampling, sequencing/composing, ADSR (shaping), pitching, 15 effects, recording, and saving both finished and active projects to disk.

## how we built it
audio backend built from scratch with cpal (rust audio library), interface built with ratatui (rust crate for tuis) using crossterm for terminal control.

## challenges we ran into
- we had to architect lots of abstractions to manage the many layers of music production packed onto a tiny device. a pocket operator aims to handle a complex task with a very, very simple interface. thus, every element must be flexible and wear many different hats.
- thus it was also challenging to make the ui intuitive - each thing has so many possible purposes that it's infeasible to label everything explicitly. discovery of the pocket operator and our tui really happen through playing around with it for a few minutes. we wanted to make context-switching and learning of the interface as seamless and fast as possible.
- staying true to the pocket operator's functionality but also extending it and making our own design choices to provide a better experience, from our own opinions as music producers/lovers

## accomplishments that we're proud of
- the amount of features we were able to replicate and extend from a device we're huge fans of :)
- designing a beautiful, minimal, and super-smooth terminal ui (with cute valentine's and treehacks easter eggs)
- creating a tool we will be using heavily ourselves and having a blast

## what we learned
- above all, this was our first time building something in rust. we learned a lot about not just rust's syntax but its compiler's specialties, design choices, and philosophy
- building terminal UIs, designing great keyboard-based interfaces that won't make people miss GUIs
- audio processing and i/o from scratch

## what's next for pocketty
- building theme-specific pockettys (like teenage engineering's lineup) and assembling a collection. each model is special. we implemented the PO-33 "K.O!" ("knockout!", it's hiphop themed)
- a manual for keybinds and capabilities, especially for those not yet experienced in music production
- cute ascii animation generation for your songs

template from devpost
