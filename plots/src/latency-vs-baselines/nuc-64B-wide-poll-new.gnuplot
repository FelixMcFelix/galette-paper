set datafile separator ","

load "gnuplot-palettes/inferno.pal"

set xlabel "Ingest Rate (\\unit{\\mega\\bit\\per\\second})"
set ylabel "Macswap Latency (\\unit{\\micro\\second})"

set key textcolor rgb "black"
set tics textcolor rgb "black"
set label textcolor rgb "black"

set tics nomirror

# set logscale y
set grid y noxtics

set key samplen 2 spacing 1 width -2 above

src_files = 4
used_rates = 6

# Example results files: (64 vs 1518B are on different rows)
array PktSizeLines[2]
PktSizeLines[1] = 1 #64
PktSizeLines[2] = 7 #1518

array SrcFiles[4]
SrcFiles[1] = "../results/SUMMARY/poll-user-nuc/2023-01-07T08.42.01+00.00/nuc/tru-kxdp-0.5p-0ms/l-summary-%sM.csv"
SrcFiles[2] = "../results/SUMMARY/poll-user-nuc/2023-01-07T08.42.01+00.00/nuc/tru-uxdp-0.5p-0ms/l-summary-%sM.csv"
SrcFiles[3] = "../results/SUMMARY/baseline-nuc/2023-01-05T09.50.11+00.00/nuc/testpmd-afp/l-summary-%sM.csv"
SrcFiles[4] = "../results/SUMMARY/baseline-nuc/2023-01-05T09.50.11+00.00/nuc/testpmd-dpdk/l-summary-0.1M.csv"

array Titles[4]
Titles[1] = "\\textsc{Gal}---XDP (Poll)"
Titles[2] = "\\textsc{Gal}---\\texttt{AF\\_XDP} (Poll)"
Titles[3] = "\\texttt{AF\\_PACKET}"
Titles[4] = "DPDK"

array LineStyles[6]
LineStyles[1] = 4
LineStyles[2] = 5
LineStyles[3] = 6
LineStyles[4] = 7
LineStyles[5] = 8
LineStyles[6] = 9

array DashStyles[6]
DashStyles[1] = 1
DashStyles[2] = 2
DashStyles[3] = 3
DashStyles[4] = 4
DashStyles[5] = 5
DashStyles[6] = 6

array Rates[7]
Rates[1] = "0.1"
Rates[2] = "0.5"
Rates[3] = "1"
Rates[4] = "10"
Rates[5] = "50"
Rates[6] = "100"
Rates[7] = "1000"

# myTitle(i) = sprintf("%s Mbps", Rates[i + 1])
# singleTitle(i) = sprintf("\\emph{\\Indfw} (\\SI{%d}{\\bit})", BitDepths[i + 1])

file(i) = sprintf(SrcFiles[(i % src_files)+1], Rates[floor(i / src_files) + 1])

xcoord(i) = 1.0 + (i * 1.0) + floor((i * 1.0) / src_files)

row(i) = 1# PktSizeLines[floor(i / src_files) + 1]

set bars 1.0
set boxwidth 1.0
set style fill empty

set xrange [0.0:(used_rates * (src_files + 1.0))]

# set xtics ("64" 2.5, \
# 	"1518" 7.5\
# 	) scale 0.0

set xtics () # clear all tics
set for [i=0:used_rates - 1] xtics add (sprintf("%s", Rates[i + 1]) ((0.5 + src_files / 2) + i * (src_files + 1)))

# candlesticks, THEN medians
# x coord, open, low, high, close
plot for [i=0:(src_files-1)] file(i) every ::row(i)::row(i) using (xcoord(i)):(1.0e-3 * $6):(1.0e-3 * $4):(1.0e-3 * $10):(1.0e-3 * $8) with candlesticks ls LineStyles[(i % src_files) + 1] fill pattern (2 + (i % src_files)) title Titles[(i % src_files) + 1] whiskerbars 0.5, \
	for [i=src_files:((used_rates * src_files) - 1)] file(i) every ::row(i)::row(i) using (xcoord(i)):(1.0e-3 * $6):(1.0e-3 * $4):(1.0e-3 * $10):(1.0e-3 * $8) with candlesticks ls LineStyles[(i % src_files) + 1] fill pattern (2 + (i % src_files)) notitle whiskerbars 0.5, \
	for [i=0:((used_rates * src_files) - 1)] file(i) every ::row(i)::row(i) using (xcoord(i)):(1.0e-3 * $7):(1.0e-3 * $7):(1.0e-3 * $7):(1.0e-3 * $7) with candlesticks lt -1 notitle

# plot for [i=0:5] file(i) u 1:3 with linespoints title myTitle(i) ls LineStyles[i + 1] dt DashStyles[1], \
# 	for [i=0:5] file(i) u 1:4 with linespoints ls LineStyles[i + 1] dt 2 notitle

