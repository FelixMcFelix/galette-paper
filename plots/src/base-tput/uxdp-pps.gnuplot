load "gnuplot-palettes/parula.pal"
set datafile separator " "

set xlabel "Packet Size"
set ylabel "Traffic Processing Rate (User-XDP, RPi) (p\\unit{\\per\\second})"

set key textcolor rgb "black"
set tics textcolor rgb "black"
set label textcolor rgb "black"

set logscale y

set key samplen 2 spacing 1 width -2 above

array BitDepths[3]
BitDepths[1] = 8
BitDepths[2] = 16
BitDepths[3] = 32

array LineStyles[6]
LineStyles[1] = 3
LineStyles[2] = 4
LineStyles[3] = 5
LineStyles[4] = 6
LineStyles[5] = 7
LineStyles[6] = 8

array DashStyles[6]
DashStyles[1] = 1
DashStyles[2] = 2
DashStyles[3] = 3
DashStyles[4] = 4
DashStyles[5] = 5
DashStyles[6] = 6

array Singles[3]
Singles[1] = 241.173
Singles[2] = 240.333
Singles[3] = 230.840

array Rates[6]
Rates[1] = "0.1"
Rates[2] = "0.5"
Rates[3] = "1"
Rates[4] = "10"
Rates[5] = "50"
Rates[6] = "100"

myTitle(i) = sprintf("%s Mbps", Rates[i + 1])
# singleTitle(i) = sprintf("\\emph{\\Indfw} (\\SI{%d}{\\bit})", BitDepths[i + 1])

file(i) = sprintf("../results/2022-11-29/t-summary-%sM-uxdp-rpi", Rates[i + 1])

plot for [i=0:5] file(i) u 1:3 with linespoints title myTitle(i) ls LineStyles[i + 1] dt DashStyles[1], \
	for [i=0:5] file(i) u 1:4 with linespoints ls LineStyles[i + 1] dt 2 notitle
