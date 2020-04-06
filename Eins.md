# Uno (Eins)

108 Karten insgesamt
Spielkarten:
-Farben:
    -Rot
    -Blau
    -Orange
    -Grün
-Symbole:
    -0
    -1
    -2
    -3
    -4
    -5
    -6
    -7
    -8
    -9
    -+2
    -+4
    -Wild(ChangeColour)
    -Reverse(ChangeDirection)
    -Skip
-Kategorie:
    -Number
    -Draw
    -ChangeNextColor
    -Skip
-DrawAction:
    -DrawTwo
    -DrawFour

## StateMachines:
-Server (Starting up, Setting up the Deck etc., Starting a new Game Session)
-Game (Setup (number of players), Begin (giving out cards), Start(random who starts if new; otherwise move clockwise), Progress (Rounds State Machine), End(Show Stats)
-Rounds ()
-Turn