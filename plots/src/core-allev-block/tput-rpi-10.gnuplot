load "gnuplot-palettes/inferno.pal"
set datafile separator ","

set xlabel "Userland Cores"
set ylabel "Throughput (\\unit{\\mega\\bit\\per\\second})"

# set logscale y

# set key top left
set key samplen 2 spacing 1 width -2 above

array LineStyles[3]
LineStyles[1] = 3
LineStyles[2] = 5
LineStyles[3] = 7

array DashStyles[2]
DashStyles[1] = 1
DashStyles[2] = 2

array Machines[2]
Machines[1] = "RPi"
Machines[2] = "NUC"

array Rates[3]
Rates[1] = "1"
Rates[2] = "10"
Rates[3] = "100"

array RateIdx[3]
RateIdx[1] = 1
RateIdx[2] = 2
RateIdx[3] = 3

# "../results/SUMMARY/VARY-P/nuc-varyall.csv"

# 4 pkt szs => 1 stride
# 5 rates   => 4 stride
# 6 ps      => 20 stride

## INFO BLOCK FOR XDP vs UPCALL.
# CoreCts tied to machine
array CoreCnts[2]
CoreCnts[1] = 3
CoreCnts[2] = 4

array PktCnts[2]
PktCnts[1] = 4
PktCnts[2] = 4

array RateCnts[2]
RateCnts[1] = 4
RateCnts[2] = 4

n_ps = 6

array StridePktSz[2]
StridePktSz[1] = CoreCnts[1]
StridePktSz[2] = CoreCnts[2]

array StrideRates[2]
StrideRates[1] = PktCnts[1] * StridePktSz[1]
StrideRates[2] = PktCnts[2] * StridePktSz[2]

array StridePs[2]
StridePs[1] = StrideRates[1] * RateCnts[1]
StridePs[2] = StrideRates[2] * RateCnts[2]

##

myTitle(i, file_idx) = sprintf("%s$\\uparrow$ (\\qty{%s}{\\mega\\bit\\per\\second})", Machines[file_idx], Rates[i + 1])

array Files[2]
Files[1] = "../results/SUMMARY/VARY-C/rpi-varyall-upcall.csv"
Files[2] = "../results/SUMMARY/VARY-C/nuc-varyall-upcall.csv"

# Median in 5, 99th in 6. T'put in 7

pkt_sz_idx = 0
target_p_idx = 4

start_pt(i, file_idx) = 1 + (target_p_idx * StridePs[file_idx]) + (RateIdx[i + 1] * StrideRates[file_idx]) + (pkt_sz_idx * StridePktSz[file_idx])
# end_pt(i, file_idx) = start_pt(i, file_idx) + CoreCnts[file_idx] - 1

plot for [i=1:1] Files[1] u 4:7 every ::(start_pt(i, 1))::(start_pt(i, 1) + CoreCnts[1] - 1) with linespoints title myTitle(i, 1) ls LineStyles[i+1] dt DashStyles[1]