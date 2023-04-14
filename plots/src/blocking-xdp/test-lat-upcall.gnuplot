load "gnuplot-palettes/inferno.pal"
set datafile separator ","

set xlabel "P(Expensive NF)"
set ylabel "Percentile Latency (\\unit{\\micro\\second})"

set logscale y

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

array Machines[2]
Machines[1] = "RPi"
Machines[2] = "NUC"

# "../results/SUMMARY/VARY-P/nuc-varyall.csv"

# 4 pkt szs => 1 stride
# 5 rates   => 4 stride
# 6 ps      => 20 stride

n_pkts = 4
n_rates = 4
n_ps = 6

stride_rate = n_pkts
stride_p = stride_rate * n_rates

myTitle(i) = sprintf("%s Median (%s)", Machines[floor(i/2) + 1], Variants[(i % 2) + 1])
myTitle2(i) = sprintf("%s 99th (%s)", Machines[floor(i/2) + 1], Variants[(i % 2) + 1])

array Files[2]
Files[1] = "../results/SUMMARY/VARY-P/rpi-varyall-upcall.csv"
Files[2] = "../results/SUMMARY/VARY-P/nuc-varyall-upcall.csv"

file(n) = Files[floor(n / 2) + 1]

plot for [i=0:3] file(i) u 1:(1.0e-3 * $4) every stride_p::(1 + stride_rate + stride_rate * (i % 2)) with linespoints title myTitle(i) ls (i) dt DashStyles[(i % 2) + 1], \
	for [i=0:3] file(i) u 1:(1.0e-3 * $5) every stride_p::(1 + stride_rate + stride_rate * (i % 2)) with linespoints title myTitle2(i) ls (i + 4) dt DashStyles[(i % 2) + 1]