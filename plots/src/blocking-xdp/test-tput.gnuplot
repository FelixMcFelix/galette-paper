load "gnuplot-palettes/inferno.pal"
set datafile separator ","

set xlabel "P(Expensive NF)"
set ylabel "Throughput (\\unit{\\mega\\bit\\per\\second})"

set key top left

array Names[2]
Names[1] = "balanced"
# Names[2] = "single"

array PresentationNames[2]
PresentationNames[1] = "Parallel"
# PresentationNames[2] = "Single Core"

array Variants[2]
Variants[1] = "\\qty{1}{\\mega\\bit\\per\\second}"
Variants[2] = "\\qty{10}{\\mega\\bit\\per\\second}"

array TargetColumn[2]
TargetColumn[1] = 4
TargetColumn[2] = 5

array LineStyles[2]
LineStyles[1] = 3
LineStyles[2] = 7

array DashStyles[2]
DashStyles[1] = 1
DashStyles[2] = 2

# "../results/SUMMARY/VARY-P/nuc-varyall.csv"

# 7 pkt szs => 1 stride
# 6 rates   => 7 stride
# 6 ps      => 42 stride

array Machines[2]
Machines[1] = "RPi"
Machines[2] = "NUC"

myTitle(i) = sprintf("%s Median (%s)", Machines[floor(i/2) + 1], Variants[(i % 2) + 1])

array Files[2]
Files[1] = "../results/SUMMARY/VARY-P/rpi-varyall.csv"
Files[2] = "../results/SUMMARY/VARY-P/nuc-varyall.csv"

file(n) = Files[floor(n / 2) + 1]

plot for [i=0:3] file(i) u 1:6 every 42::(1 + 14 + 7 * (i % 2)) with linespoints title myTitle(i) ls (i) dt DashStyles[(i % 2) + 1]