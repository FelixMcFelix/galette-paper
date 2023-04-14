load "gnuplot-palettes/inferno.pal"
set datafile separator ","

set xlabel "P(Expensive NF)"
set ylabel "Percentile Latency (\\unit{\\micro\\second})"

# set logscale y

# set key top left
set key samplen 2 spacing 1 width -2 above

array Names[2]
Names[1] = "balanced"
# Names[2] = "single"

array PresentationNames[2]
PresentationNames[1] = "Parallel"
# PresentationNames[2] = "Single Core"

array LineStyles[2]
LineStyles[1] = 3
LineStyles[2] = 7

array DashStyles[2]
DashStyles[1] = 1
DashStyles[2] = 2

array Machines[2]
Machines[1] = "RPi"
Machines[2] = "NUC"

array Rates[2]
Rates[1] = "1"
Rates[2] = "100"

array RateIdxXdp[2]
RateIdxXdp[1] = 2
RateIdxXdp[2] = 5

array RateIdxUpcall[2]
RateIdxUpcall[1] = 1
RateIdxUpcall[2] = 3

# "../results/SUMMARY/VARY-P/nuc-varyall.csv"

# 4 pkt szs => 1 stride
# 5 rates   => 4 stride
# 6 ps      => 20 stride

## INFO BLOCK FOR XDP vs UPCALL.
array PktCnts[2]
PktCnts[1] = 7
PktCnts[2] = 4

array RateCnts[2]
RateCnts[1] = 6
RateCnts[2] = 4

n_ps = 6

array StrideRates[2]
StrideRates[1] = PktCnts[1]
StrideRates[2] = PktCnts[2]

array StridePs[2]
StridePs[1] = StrideRates[1] * RateCnts[1]
StridePs[2] = StrideRates[2] * RateCnts[2]

##

array Quals[2]
Quals[1] = ""
Quals[2] = "$\\uparrow$"

myTitle(i) = sprintf("%s%s (\\qty{%s}{\\mega\\bit\\per\\second})", Machines[(i%2) + 1], Quals[floor(i / 2) + 1], Rates[(i%2) + 1])

array Files[4]
Files[1] = "../results/SUMMARY/VARY-P/rpi-varyall.csv"
Files[2] = "../results/SUMMARY/VARY-P/nuc-varyall.csv"
Files[3] = "../results/SUMMARY/VARY-P/rpi-varyall-upcall.csv"
Files[4] = "../results/SUMMARY/VARY-P/nuc-varyall-upcall.csv"

file(n) = Files[n + 1]

# Median in 4, 99th in 5.
plot for [i=0:1] file(i) u 1:(1.0e-3 * $5) every StridePs[1]::(1 + StrideRates[1] * RateIdxXdp[i + 1]) with linespoints title myTitle(i) ls LineStyles[i+1] dt DashStyles[1], \
	for [i=0:1] file(2+i) u 1:(1.0e-3 * $5) every StridePs[2]::(1 + StrideRates[2] * RateIdxUpcall[i + 1]) with linespoints title myTitle(2+i) ls LineStyles[i+1] dt DashStyles[2]