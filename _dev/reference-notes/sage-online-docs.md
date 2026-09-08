# Sage Documentation

_Source folder: `sage-docs-md`_


## docs/configuration

## JSON file schema – Nextra

Source: https://sage-docs.vercel.app/docs/configuration

DocumentationConfiguring Sage# Configuring Sage

Sage is primarily configured via a single file , but there are several parameters that can be set or overrode by command-line arguments or environment variables:

### Command Line Arguments

- --batch-size N : Process N files in parallel. Setting this value to the number of logical CPU cores will maximize performance at the cost of higher memory use
- --write-pin : Write percolator/mokapot compatible output file in addition to any other output files
- --parquet : Write parquet-formatted files instead of tab-separated files. Do not use this if you need to open your results in Excel.
- --fasta path : Override or set path to FASTA database
- --output_directory : Override or set path to output directory or S3 location
- --annotate-matches : Record all experimental-theoretical fragment ion matches for use in generating spectral libraries or rescoring PSMs
- --disable-telemetry-i-dont-want-to-improve-sage : Turn off Sage's basic telemetry (which reports # of CPU cores, memory usage, run time, and size of fragment ion index)
- [mzml_paths] : Override or set mzML files to search

```
Usage:
 
sage
 [OPTIONS] 
<
parameters
>
 [mzml_paths]...

 

🔮
 
Sage
 
🧙
 
-
 
Proteomics
 
searching
 
so
 
fast
 
it
 
feels
 
like
 
magic!

 

Arguments:

  
<
parameters>
     
Path
 
to
 
configuration
 
parameters
 (JSON 
file
)

  [mzml_paths]
...
  
Paths
 
to
 
mzML
 
files
 
to
 
process.
 
Overrides
 
mzML
 
files
 
listed
 
in
 
the
 
configuration
 
file.

 

Options:

  
-f,
 
--fasta
 
<
fast
a
>

          
Path
 
to
 
FASTA
 
database.
 
Overrides
 
the
 
FASTA
 
file
 
specified
 
in
 
the
 
configuration
 
file.

  
-o,
 
--output_directory
 
<
output_director
y
>

          
Path
 
where
 
search
 
and
 
quant
 
results
 
will
 
be
 
written.
 
Overrides
 
the
 
directory
 
specified
 
in
 
the
 
configuration
 
file.

      
--batch-size
 
<
batch-siz
e
>

          
Number
 
of
 
files
 
to
 
search
 
in
 
parallel
 (default 
=
 
number
 
of
 
CPUs/2
)

      
--parquet

          
Write
 
parquet
 
files
 
instead
 
of
 
tab-separated
 
files

      
--annotate-matches

          
Write
 
matched
 
fragments
 
output
 
file.

      
--write-pin

          
Write
 
percolator-compatible
 
`
.pin
`
 
output
 
files

      
--disable-telemetry-i-dont-want-to-improve-sage

          
Disable
 
sending
 
telemetry
 
data

  
-h,
 
--help

          
Print
 
help
 
information

  
-V,
 
--version

          
Print
 
version
 
information
```

### Environment Variables

There are two shell environment variables that can be used to change behavior of Sage:

```
# Sage will print additional information to the terminal

export
 SAGE_LOG
=
trace

# Sage will only use N threads, default is to use the number of logical CPU cores

export
 RAYON_NUM_THREADS
=
2

 

sage
 
config.json
```

or in inline-style:

```
SAGE_LOG
=
trace
 RAYON_NUM_THREADS
=
2
 
sage
 
config.json
```

### JSON File Schema

Sage uses a json file for configuration. This enables easy programmatic manipulation of the configuration file. The order of options in the configuration file does not matter.

A full example is shown below, but it's worth noting that many of the configuration options can be omitted the first time you run Sage.

One of the nice features of Sage is that it will write out a results.json file listing all parameters that were actually used during the run (i.e. any defaults that you did not specify),
which you can then modify. An even nicer feature is that you can reproduce your results by running the results.json file!

```
# Reproduce the same results!

sage
 
results.json
```

⬅️ Please checkout the rest of the docs for detailed descriptions of the parameters

🚫Note that json files do not allow comments - they are provided here for explanation, but need to be removed in a real configuration file. Trailing commas will also cause an error to be thrown.
You can check out some example files without comments in the sidebar ( Example pages)

config.json```
{

  
"database"
:
 {

    
// How many fragments are in each internal mass bucket

    
// Use a lower value (8192) for high-res MS/MS, and higher values for low-res MS/MS

    
"bucket_size"
:
 
32768
,

    
// This section is optional. Default is trypsin, using the parameters below

    
"enzyme"
:
 {

      
// Optional[int] {default=1}, Number of missed cleavages to allow

      
"missed_cleavages"
:
 
2
,

      
// Optional[int] {default=5}, Minimum AA length of peptides to search

      
"min_len"
:
 
5
,

      
// Optional[int] {default=50}, Maximum AA length of peptides to search

      
"max_len"
:
 
50
,

      
// Optional[str] {default='KR'}. Amino acids to cleave at

      
"cleave_at"
:
 
"KR"
,

      
// Optional[char/single AA] {default='P'}. Do not cleave if this AA follows the cleavage site

      
"restrict"
:
 
"P"
,

      
// Optional[bool] {default=true}. Cleave at c terminus of matching amino acid

      
"c_terminal"
:
 
true

    }
,

    
// Optional[float] {default=500.0}, Minimum monoisotopic mass of peptides to fragment

    
"peptide_min_mass"
:
 
500.0
,

    
// Optional[float] {default=5000.0}, Maximum monoisotopic mass of peptides to fragment

    
"peptide_max_mass"
:
 
5000.0
,

    
// Optional[List[str]] {default=["b","y"]} Which fragment ions to generate and search?

    
"ion_kinds"
:
 [
"b"
,
 
"y"
]
,

    
// Optional[int] {default=2}, Do not generate b1/b2/y1/y2 ions for preliminary searching.

    
// Does not affect full scoring of PSMs!

    
"min_ion_index"
:
 
2
,

    
// Optional[Dict[char, float]] {default=null}, static modifications

    
"static_mods"
:
 {

      
// Apply static modification to N-terminus of peptide

      
"^"
:
 
304.207
,

      
// Apply static modification to lysine

      
"K"
:
 
304.207
,

      
// Apply static modification to cysteine

      
"C"
:
 
57.0215

    }
,

    
// Optional[Dict[char, list[float]]] {default=null}, variable modifications

    
"variable_mods"
:
 {

      
// Variable mods are applied *before* static mod

      
"M"
:
 [
15.9949
]
,

      
"^Q"
:
 [
-17.026549
]
,

      
// Applied to N-terminal glutamic acid

      
"^E"
:
 [
-18.010565
]
,

      
// Applied to peptide C-terminus

      
"$"
:
 [
49.2
,
 
22.9
]
,

      
// Applied to protein N-terminus

      
"["
:
 
42.0
,

      
// Applied to protein C-terminus

      
"]"
:
 
111.0

    }
,

    
// Optional[int] {default=2} Limit k-combinations of variable modifications

    
"max_variable_mods"
:
 
2
,

    
// Optional[str] {default="rev_"}: See notes above

    
"decoy_tag"
:
 
"rev_"
,

    
// Optional[bool] {default="true"}: Ignore decoys in FASTA database matching `decoy_tag`

    
"generate_decoys"
:
 
false
,

    
// str: mandatory path to FASTA file

    
"fasta"
:
 
"dual.fasta"

  }
,

  
// Optional - specify only if TMT or LFQ

  
"quant"
:
 {

    
// Optional[str] {default=null}, one of "Tmt6", "Tmt10", "Tmt11", "Tmt16", or "Tmt18"

    
"tmt"
:
 
"Tmt16"
,

    
"tmt_settings"
:
 {

      
// Optional[int] {default=3}, MS-level to perform TMT quantification on

      
"level"
:
 
3
,

      
// Optional[bool] {default=false}, use Signal/Noise instead of intensity for TMT quant

      
// Requires noise values in mzML

      
"sn"
:
 
false

    }
,

    
// Optional[bool] {default=null}, perform label-free quantification

    
"lfq"
:
 
true
,

    
"lfq_settings"
:
 {

      
// See documentation for details - recommend that you do not change this setting

      
"peak_scoring"
:
 
"Hybrid"
,

      
// Optional["Sum" | "Apex"] {default="Sum"}, use sum or peak of MS1 traces in peak

      
"integration"
:
 
"Sum"
,

      
// Optional[float] {default = 0.7}, normalized spectral angle (vs. theoretical isotopic envelope)

      
// cutoff for calling an MS1 peak

      
"spectral_angle"
:
 
0.7
,

      
// Optional[float] {default = 5.0}, tolerance for DICE window around calculated precursor mass

      
"ppm_tolerance"
:
 
5.0
,

      
// Optional[float] {default = 3.0}, tolerance for DICE window around observed precursor mobility

      
"mobility_pct_tolerance"
:
 
3.0

    }

  }
,

  
// Tolerance can be either "ppm" or "da"

  
"precursor_tol"
:
 {

    
"da"
:
 [

      
// This value is substracted from the experimental precursor to match theoretical peptides

      
-500
,

      
// This value is added to the experimental precursor to match theoretical peptides

      
100

    ]

  }
,

  
"fragment_tol"
:
 {

    
"ppm"
:
 [

     
// This value is subtracted from the experimental fragment to match theoretical fragments 

     
-10
,

     
// This value is added to the experimental fragment to match theoretical fragments 

     
10

    ]

  }
,

  
// Optional[Tuple[int, int]] {default=[2, 4]}

  
// If charge states are not annotated in the mzML, or if `wide_window` mode is turned on, then consider

  
// all precursors at z=2, z=3, z=4

  
"precursor_charge"
:
 [
2
,
 
4
]
,

  
// Optional[Tuple[int, int]] {default=[0,0]}: C13 isotopic envelope to consider for precursor

  
"isotope_errors"
:
 [

    
// Consider -1 C13 isotope

    
-1
,

    
// Consider up to +3 C13 isotope (-1/0/1/2/3) 

    
3

  ]
,

  
// Optional[bool] {default=false}: perform deisotoping and charge state deconvolution on MS2 spectra

  
"deisotope"
:
 
false
,

  
// Optional[bool] {default=false}: search for chimeric/co-fragmenting PSMs

  
"chimera"
:
 
false
,

  
// Optional[bool] {default=false}: _ignore_ `precursor_tol` and search in wide-window/DIA mode

  
"wide_window"
:
 
false
,

  
// Optional[bool] {default=true}: use retention time prediction model as an feature for LDA

  
"predict_rt"
:
 
false
,

  
// Optional[int] {default=15}: only process MS2 spectra with at least N peaks

  
"min_peaks"
:
 
15
,

  
// Optional[int] {default=150}: take the top N most intense MS2 peaks to search,

  
"max_peaks"
:
 
150
,

  
// Optional[int] {default=4}: minimum # of matched b+y ions to use for reporting PSMs

  
"min_matched_peaks"
:
 
6
,

  
// Optional[int] {default=null}: maximum fragment ion charge states to consider,

  
"max_fragment_charge"
:
 
1
,

  
// Optional[int] {default=1}: number of PSMs to report for each spectra. Higher values might disrupt PSM rescoring.

  
"report_psms"
:
 
1
,

  
// Optional[str] {default=`.`}: Place output files in a given directory or S3 bucket/prefix

  
"output_directory"
:
 
"s3://bucket/prefix"
,

  
// List[str]: representing paths to mzML (or gzipped-mzML) files for search

  
"mzml_paths"
:
 [

    
"local/path.mzML"
,

    
"s3://bucket/PXD0000001/foo.mzML.gz"
,

    
"local/path.d"
 
// Sage can also read natively bruker .d files.

  ]
,

  
// Configuration to process Bruker data read from .d

  
"bruker_config"
:
 {

    
"ms2"
:
 {

      
// Optional configuration to process the frames into spectra.

      
// Used for both DDA and DIA

      
"spectrum_processing_params"
:
 {

        
"smoothing_window"
:
 
1
,

        
"centroiding_window"
:
 
1
,

        
"calibration_tolerance"
:
 
0.1
,

        
"calibrate"
:
 
false

      }
,

      
// Optional configuration to split MS2 frames.

      
// Only used if when reading DIA data.

      
"frame_splitting_params"
:
 {

        
"Quadrupole"
:
 {

          
"UniformMobility"
:
 [[
0.1
,
 
0.05
]
,
 
null
]

        }

      }

    }
,

    
"ms1"
:
 {

      
// Optional configuration to centroid MS1 frames.

      
// Only used if LFQ is enabled AND bruker data is read.

      
"mz_ppm"
:
 
15.0
,

      
"mobility_pct"
:
 
3.0

    }

  }

 

}
```

How Sage WorksFragment Index Construction


## docs/configuration/additional

## Chimeric search, RT prediction, misc. – Nextra

Source: https://sage-docs.vercel.app/docs/configuration/additional

DocumentationConfiguring SageChimeric search, RT prediction, misc.# Additional Settings

### Chimeric search and reporting multiple PSMs

Sage is capable of searching for chimeric/co-fragmenting PSMs in any search configuration. This feature requires that report_psms > 1 to have any special effect.
During the search phase, Sage will find the best candidate peptide for a spectrum (highest hyperscore), and then subtract the most intense MS2 peak matching each
fragment ion from that peptide. Sage will then search the subtracted spectrum again, and repeat the above step until report_psms candidates have been found for the
spectrum, or there are fewer than min_peaks MS2 peaks remaining in the spectra.

- chimera : Boolean. Search for chimeric/co-fragmenting PSMs (default: false).
- report_psms : Integer. The number of PSMs to report for each spectrum. Higher values might disrupt re-scoring, it is best to search with multiple values (default: 1).

### Retention Time Prediction

💡You probably don't want to turn this off without good reason!

Sage's LFQ module requires that global RT alignment and prediction is performed, and will turn on this feature even if you turn it off.

By default, Sage will align retention times across all files in the search by matching RTs for the most confident PSMs shared between runs.

Global retention time alignment is performed using a modified algorithm based on Chen AT, et al. (opens in a new tab)

1. Transform all RTs into unit-less percentages (0.0 - 1.0)
2. Assume that the expected RT for a peptide can be estimated from the average
RT across all runs
3. For each run, calculate a linear regression between the observed peptide RTs
and the global average. Transform all PSM retention times by the regression
parameters
4. Following global RT alignment, Sage will then build a RT prediction model, and integrate this information as a feature for PSM rescoring.

See Klammer et al. (opens in a new tab)

If, for some reason, you would prefer the above to be turned off, you can do via the predict_rt flag.

```
{

    
// Oh no!

    
"predict_rt"
:
 
false

}
```

QuantificationInterfacing with AWS S3


## docs/configuration/aws

## Interfacing with AWS S3 – Nextra

Source: https://sage-docs.vercel.app/docs/configuration/aws

DocumentationConfiguring SageInterfacing with AWS S3# Interfacing with AWS S3

Using S3 may incur data transfer charges as well as multi-part upload request charges.

Sage is capable of natively reading & writing files located on AWS S3 (or S3-compatible systems, such as minio).

- S3 paths should be specified as s3://bucket/prefix/key.mzML.gz or s3://bucket/prefix for output folder. This can be used for any path passed to Sage (configuration file, FASTA location, etc)
- See AWS docs (opens in a new tab) for configuring your credentials

Chimeric search, RT prediction, misc.Example: PXD003881 (Label-free quant)


## docs/configuration/bruker

## Bruker File Processing – Nextra

Source: https://sage-docs.vercel.app/docs/configuration/bruker

DocumentationConfiguring SageBruker File Processing# Bruker-Specific processing

Several parameters can change how the bruker data is processed.

```
{

  
"bruker_config"
:
 {

    
"ms1"
:
 {

      
"mz_ppm"
:
 
15.0
,

      
"ims_pct"
:
 
3.0

    }
,

    
"ms2"
:
 {

      
"spectrum_processing_params"
:
 {

        
"smoothing_window"
:
 
1
,

        
"centroiding_window"
:
 
1
,

        
"calibration_tolerance"
:
 
0.1
,

        
"calibrate"
:
 
false

      }
,

      
"frame_splitting_params"
:
 {

        
"Quadrupole"
:
 {

          
"UniformMobility"
:
 [[
0.1
,
 
0.05
]
,
 
null
]

        }

      }
,
  

    }

  }

}
```

### MS1

These parameters modify the tolerances to centroid peaks along the mobility
dimension of a frame.

This parameter is only used if the lfq option is enabled.

centroid_config```
{

  
"bruker_config"
:
 {

    
"ms1"
:
 {

      
"mz_ppm"
:
 
15.0
,

      
"ims_pct"
:
 
3.0

    }

}
```

## MS2 Spectrum Processing Parameters

Default Parameters```
{

  
"bruker_config"
:
 {

    
"ms2"
:
 {

      
"spectrum_processing_params"
:
 {

        
"smoothing_window"
:
 
1
,

        
"centroiding_window"
:
 
1
,

        
"calibration_tolerance"
:
 
0.1
,

        
"calibrate"
:
 
false

      }

    }

  }

}
```

### (Experimental) Frame Splitting Parameters

These parameters can be used to search timsDIA data using Sage.
In essence they describe how the frame should be split to generate
spectra.

There are two major options that describe what subsets of the frame
should be considered an unit to split by; The options are Window and Quadrupole . And Three minor options, null , UniformMobility ,
and Even .

1. null , No splitting beyond the major group is done.
2. UniformMobility , Splits each region in areas of a specified size,
and that overlap by a specific value. For instance [0.1, 0.05]
will combine regions of size 0.1 1/k0 and will overlap by 0.05 with
each other.
3. Even Will evenly divide the mobility window into N number of
groups.

Default value :

```
{

  
"bruker_config"
:
 {

    
"ms2"
:
 {

      
"frame_splitting_params"
:
 {

        
// "Window": { // All internal options are also compatible with `Window`

        
"Quadrupole"
:
 {

          
"UniformMobility"
:
 [[
0.1
,
 
0.05
]
,
 
null
]

          
// "Even": 10

          
// null 

        }

      }

    }

  }

}
```

Example: PXD001468 (Open-Search)Interpreting Sage Results


## docs/configuration/database

## Fragment Index Construction – Nextra

Source: https://sage-docs.vercel.app/docs/configuration/database

DocumentationConfiguring SageFragment Index Construction# Fragment Index Construction

The database section of the configuration file determines how the internal fragment index data structure is generated.

```
{

    
"bucket_size"
:
 
8192
,

    
"enzyme"
:
 {

        
"missed_cleavages"
:
 
2
,

        
"min_len"
:
 
7
,

        
"max_len"
:
 
50
,

        
"cleave_at"
:
 
"KR"
,

        
"restrict"
:
 
"P"
,

        
"c_terminal"
:
 
true
,

        
"semi_enzymatic"
:
 
false

    }
,

    
"peptide_min_mass"
:
 
500.0
,

    
"peptide_max_mass"
:
 
5000.0
,

    
"ion_kinds"
:
 [

        
"b"
,

        
"y"

    ]
,

    
"min_ion_index"
:
 
2
,

    
"max_variable_mods"
:
 
2
,

    
"static_mods"
:
 {

        
"C"
:
 
57.0214

    }
,

    
"variable_mods"
:
 {

        
"M"
:
 
15.9949

    }
,

    
"decoy_tag"
:
 
"rev_"
,

    
"generate_decoys"
:
 
true
,

    
"fasta"
:
 
"s3://sage-benchmarking/fasta/human_contam.fasta"

}
```

### Bucket size

This parameter only affects search speed and will not change your results.

| MS2 resolution | Suggested Setting |
| --- | --- |
| Low | 65536 |
| High | 8192 |

This parameter can be used to tune performance of Sage. This value sets the number of fragment ions within each "bucket" in the internal index datastructure.
This value will always be set to the next largest power of 2.

A smaller number ( 8192 is the minimum) is suitable for high resolution MS/MS spectra, since not many buckets will need to be searched. Low resolution MS/MS spectra
will need to search more buckets, so increasing the size of the bucket will lower the total number of internal buckets.
A good starting point is to use 65536 for ion-trap data, but the optimal value for your search parameters and files might require empirical tuning.

### Enzyme

The enzyme section contains parameters related to the enzyme used for digestion. The default enzyme is trypsin, with the parameters specified below.

- missed_cleavages : Integer. The number of missed cleavages for tryptic digest (default: 1).
- min_len : Integer. The minimum amino acid (AA) length of peptides to search (default: 5).
- max_len : Integer. The maximum AA length of peptides to search (default: 50).
- cleave_at : String. Amino acids to cleave at (default: 'KR'). The cleave_at parameter can also be used to specify alternative digestion schemes: Non-enzymatic: cleave_at = "" - All potential peptides between min_len and max_len will be generated from the sequence No digestion: cleave_at = "$" - FASTA entries will be used as-is, subject to min_len and max_len options
- restrict : Single character string. Do not cleave if this amino acid follows the cleavage site (default: 'P').
- c_terminal : Boolean. Cleave at the C-terminus of matching amino acids (default:true).
- semi_enzymatic : Boolean. Perform a semi-enzymatic digest (default:false).

### Fragment Settings

- peptide_min_mass : Float. The minimum monoisotopic mass of peptides to fragment in silico (default: 500.0).
- peptide_max_mass : Float. The maximum monoisotopic mass of peptides to fragment in silico (default: 5000.0).
- ion_kinds : List of strings. Which fragment ions to produce? Allowed values: "a", "b", "c", "x", "y", "z". (default: ["b", "y"])
- min_ion_index : Integer. Do not generate b1..bN or y1..yN ions for preliminary searching if min_ion_index = N . Does not affect full scoring of PSMs (default: 2).

### Modifications

#### Static Modifications

- static_mods Dictionary with characters as keys and floats as values. Represents static modifications applied to amino acids or termini (default: ). Static modifications are applied after variable modifications

Example: Apply a static modification of 304.207 to the N-terminus of the peptide and lysine, and 57.0215 to cysteine.

static mods```
{

    
"static_mods"
:
 {

        
"^"
:
 
304.207
,

        
"K"
:
 
304.207
,

        
"C"
:
 
57.0215

    }

}
```

### Variable Modifications

- max_variable_mods : Integer. Limit k-combinations of variable modifications (default: 2).
- variable_mods : Dictionary with characters as keys and list of floats (or single floats) as values. Represents variable modifications applied to amino acids or termini (default: ).

Example: Apply a variable modification of 15.9949 to methionine, 49.2022 to the C-terminus of the peptide,
42.0 to the N-terminus of the protein, and 111.0 to the C-terminus of the protein, in addition to pyro-glutamine/pyro-glutamic acid.
Allow only up to 3 variable modifications in total.

variable mods```
{

    
"max_variable_mods"
:
 
3
,

    
"variable_mods"
:
 {

        
"M"
:
 [
15.9949
]
,
 

        
"^Q"
:
 [
-17.026549
]
,

        
"^E"
:
 [
-18.010565
]
,

        
"$"
:
 [
49.2022
]
,

        
"["
:
 
42.0
,

        
"]"
:
 
111.0

    }

}
```

#### Modification Syntax:

- "^X" : Modification to be applied to amino acid X if it appears at the N-terminus of a peptide
- "$X" : Modification to be applied to amino acid X if it appears at the C-terminus of a peptide
- "[X" : Modification to be applied to amino acid X if it appears at the N-terminus of a protein
- "]X" : Modification to be applied to amino acid X if it appears at the C-terminus of a protein

### Fasta Database and Decoy Generation

💡For best results, let Sage generate decoy sequences.

- decoy_tag : String. The tag used to identify decoy entries in the FASTA database (default: "rev_").
- generate_decoys : Boolean. If true, ignore decoys in the FASTA database matching decoy_tag , and generate internally reversed peptides (default: false).
- fasta : String. The path to the FASTA file, either a local path or s3 object URI.

Target-decoy competition is key to controlling the false discovery rate in proteomics experiments.
Sage can use decoy sequences included in the supplied FASTA file, or it can generate internal sequences (recommended).
Sage reverses tryptic peptides (not proteins), so that the picked-peptide (opens in a new tab) approach to FDR can be used.

PEPTIDEK  (Click to reverse) If generate_decoys is set to true (or unspecified), then decoy sequences in the FASTA database matching decoy_tag will be ignored ,
and Sage will internally generate decoys.

🚫It is critical that you ensure you use the proper decoy_tag if you are using a FASTA database containing decoys
and have internal decoy generation turned on - otherwise Sage will treat the supplied decoys as hits!

Internally generated decoys will have protein accessions matching "{decoy_tag}{accession}" , e.g. if decoy_tag is "rev_" then a protein accession like "rev_sp|P01234|HUMAN" will be listed in the output file.

Configuring SageSearch Tolerances


## docs/configuration/example_PXD001468

## Example: PXD001468 (Open-Search) – Nextra

Source: https://sage-docs.vercel.app/docs/configuration/example_PXD001468

DocumentationConfiguring SageExample: PXD001468 (Open-Search)# Example: PXD001468 (Open-Search)

The following configuration file is an example of running a chimeric open-search on a single high-res MS/MS file.

PXD001468.json```
{

    
"database"
:
 {

        
"bucket_size"
:
 
8192
,

        
"enzyme"
:
 {

            
"missed_cleavages"
:
 
2
,

            
"min_len"
:
 
7
,

            
"max_len"
:
 
50
,

            
"cleave_at"
:
 
"KR"
,

            
"restrict"
:
 
"P"

        }
,

        
"peptide_min_mass"
:
 
500.0
,

        
"peptide_max_mass"
:
 
5000.0
,

        
"ion_kinds"
:
 [

            
"b"
,

            
"y"

        ]
,

        
"min_ion_index"
:
 
2
,

        
"max_variable_mods"
:
 
2
,

        
"static_mods"
:
 {

            
"C"
:
 
57.0214

        }
,

        
"variable_mods"
:
 {

            
"M"
:
 
15.9949

        }
,

        
"decoy_tag"
:
 
"rev_"
,

        
"generate_decoys"
:
 
true
,

        
"fasta"
:
 
"human_contam.fasta"

    }
,

    
"precursor_tol"
:
 {

        
"da"
:
 [

            
-500
,

            
100

        ]

    }
,

    
"fragment_tol"
:
 {

        
"ppm"
:
 [

            
-10
,

            
10

        ]

    }
,

    
"deisotope"
:
 
true
,

    
"chimera"
:
 
true
,

    
"min_peaks"
:
 
15
,

    
"max_peaks"
:
 
250
,

    
"min_matched_peaks"
:
 
4
,

    
"max_fragment_charge"
:
 
1
,

    
"report_psms"
:
 
2
,

    
"predict_rt"
:
 
true
,

    
"mzml_paths"
:
 [

        
"PXD001468/b1906_293T_proteinID_01A_QE3_122212.mzML.gz"

    ]

}
```

Example: PXD003881 (Label-free quant)Bruker File Processing


## docs/configuration/example_PXD003881

## Example: PXD003881 (Label-free quant) – Nextra

Source: https://sage-docs.vercel.app/docs/configuration/example_PXD003881

DocumentationConfiguring SageExample: PXD003881 (Label-free quant)# Example: PXD003881 (Label-free quant)

The following configuration file is an example of running a closed-search with label-free quantification on a set of 20 runs. These runs have E. coli lysates spiked into human cell background at known amounts.

PXD001468.json```
{

    
"database"
:
 {

        
"bucket_size"
:
 
8192
,

        
"enzyme"
:
 {

            
"missed_cleavages"
:
 
2
,

            
"min_len"
:
 
7
,

            
"max_len"
:
 
50
,

            
"cleave_at"
:
 
"KR"
,

            
"restrict"
:
 
"P"

        }
,

        
"peptide_min_mass"
:
 
500.0
,

        
"peptide_max_mass"
:
 
5000.0
,

        
"ion_kinds"
:
 [

            
"b"
,

            
"y"

        ]
,

        
"min_ion_index"
:
 
2
,

        
"max_variable_mods"
:
 
3
,

        
"static_mods"
:
 {

            
"C"
:
 
57.0215

        }
,

        
"variable_mods"
:
 {

            
"M"
:
 
15.994

        }
,

        
"decoy_tag"
:
 
"rev_"
,

        
"generate_decoys"
:
 
true
,

        
"fasta"
:
 
"PXD003881.fasta"

    }
,

    
"quant"
:
 {

        
"lfq"
:
 
true
,

        
"lfq_settings"
:
 {

            
"peak_scoring"
:
 
"Hybrid"
,

            
"integration"
:
 
"Sum"
,

            
"spectral_angle"
:
 
0.6
,

            
"ppm_tolerance"
:
 
5.0

        }

    }
,

    
"precursor_tol"
:
 {

        
"ppm"
:
 [

            
-20.0
,

            
20.0

        ]

    }
,

    
"fragment_tol"
:
 {

        
"ppm"
:
 [

            
-20.0
,

            
20.0

        ]

    }
,

    
"isotope_errors"
:
 [

        
0
,

        
2

    ]
,

    
"deisotope"
:
 
true
,

    
"min_peaks"
:
 
15
,

    
"max_peaks"
:
 
150
,

    
"max_fragment_charge"
:
 
1
,

    
"min_matched_peaks"
:
 
4
,

    
"predict_rt"
:
 
true
,

    
"mzml_paths"
:
 [

        
"PXD003881/B03_02_150304_human_ecoli_B_3ul_3um_column_95_HCD_OT_2hrs_30B_9B.mzML.gz"
,

        
"PXD003881/B03_03_150304_human_ecoli_C_3ul_3um_column_95_HCD_OT_2hrs_30B_9B.mzML.gz"
,

        
"PXD003881/B03_04_150304_human_ecoli_D_3ul_3um_column_95_HCD_OT_2hrs_30B_9B.mzML.gz"
,

        
"PXD003881/B03_05_150304_human_ecoli_E_3ul_3um_column_95_HCD_OT_2hrs_30B_9B.mzML.gz"
,

        
"PXD003881/B03_06_150304_human_ecoli_E_3ul_3um_column_95_HCD_OT_2hrs_30B_9B.mzML.gz"
,

        
"PXD003881/B03_07_150304_human_ecoli_D_3ul_3um_column_95_HCD_OT_2hrs_30B_9B.mzML.gz"
,

        
"PXD003881/B03_08_150304_human_ecoli_C_3ul_3um_column_95_HCD_OT_2hrs_30B_9B.mzML.gz"
,

        
"PXD003881/B03_09_150304_human_ecoli_B_3ul_3um_column_95_HCD_OT_2hrs_30B_9B.mzML.gz"
,

        
"PXD003881/B03_10_150304_human_ecoli_A_3ul_3um_column_95_HCD_OT_2hrs_30B_9B.mzML.gz"
,

        
"PXD003881/B03_11_150304_human_ecoli_A_3ul_3um_column_95_HCD_OT_2hrs_30B_9B.mzML.gz"
,

        
"PXD003881/B03_12_150304_human_ecoli_B_3ul_3um_column_95_HCD_OT_2hrs_30B_9B.mzML.gz"
,

        
"PXD003881/B03_13_150304_human_ecoli_C_3ul_3um_column_95_HCD_OT_2hrs_30B_9B.mzML.gz"
,

        
"PXD003881/B03_14_150304_human_ecoli_D_3ul_3um_column_95_HCD_OT_2hrs_30B_9B.mzML.gz"
,

        
"PXD003881/B03_15_150304_human_ecoli_E_3ul_3um_column_95_HCD_OT_2hrs_30B_9B.mzML.gz"
,

        
"PXD003881/B03_16_150304_human_ecoli_E_3ul_3um_column_95_HCD_OT_2hrs_30B_9B.mzML.gz"
,

        
"PXD003881/B03_17_150304_human_ecoli_D_3ul_3um_column_95_HCD_OT_2hrs_30B_9B.mzML.gz"
,

        
"PXD003881/B03_18_150304_human_ecoli_C_3ul_3um_column_95_HCD_OT_2hrs_30B_9B.mzML.gz"
,

        
"PXD003881/B03_19_150304_human_ecoli_B_3ul_3um_column_95_HCD_OT_2hrs_30B_9B.mzML.gz"
,

        
"PXD003881/B03_20_150304_human_ecoli_A_3ul_3um_column_95_HCD_OT_2hrs_30B_9B.mzML.gz"
,

        
"PXD003881/B03_21_150304_human_ecoli_A_3ul_3um_column_95_HCD_OT_2hrs_30B_9B.mzML.gz"

    ]

}
```

Interfacing with AWS S3Example: PXD001468 (Open-Search)


## docs/configuration/quantification

## Quantification – Nextra

Source: https://sage-docs.vercel.app/docs/configuration/quantification

DocumentationConfiguring SageQuantification# Quantification

💡The quant section is optional and needs to be specified only if TMT or LFQ is used.

Sage can perform fast and accurate label-free and isobaric (i.e. TMT) quantification. The following settings can be used to configure quantification:

```
{

 
"quant"
:
 {

    
"tmt"
:
 
"Tmt16"
,

    
"tmt_settings"
:
 {

      
"level"
:
 
3
,

      
"sn"
:
 
false

    }
,

    
"lfq"
:
 
true
,

    
"lfq_settings"
:
 {

      
"peak_scoring"
:
 
"Hybrid"
,

      
"integration"
:
 
"Sum"
,

      
"spectral_angle"
:
 
0.7
,

      
"ppm_tolerance"
:
 
5.0
,

      
"mobility_pct_tolerance"
:
 
3.0

    }

  }

}
```

#### TMT

- tmt : String. One of "Tmt6", "Tmt10", "Tmt11", "Tmt16", or "Tmt18" (default: null).
- tmt_settings : Object containing TMT-specific settings. level : Integer. The MS-level to perform TMT quantification on (default: 3). sn : Boolean. Use Signal/Noise instead of intensity for TMT quantification. Requires noise values in mzML (default: false).

If you need to report S/N measurements, ThermoRawFileParser (opens in a new tab) supports adding noise values to mzMLs.

#### LFQ

- lfq : Boolean. Perform label-free quantification (default: null).
- lfq_settings : Object containing LFQ-specific settings. peak_scoring : String. The method used for scoring peaks in LFQ, one of: "Hybrid", "RetentionTime", "SpectralAngle" (default: "Hybrid"). Hybrid scoring combines RT-based and spectral-angle based scoring to identify the best MS1 peak to quantify. integration : String. The method used for integrating peak intensities, either "Sum" or "Apex" (default: "Sum"). spectral_angle : Float. Threshold for the normalized spectral angle similarity measure (observed vs theoretical isotopic envelope), ranging from 0 to 1 (default: 0.7). ppm_tolerance : Float. Tolerance for matching MS1 ions in parts per million (default: 5.0). mobility_pct_tolerance : Float. Tolerance for matching MS1 ions in percent (default: 3.0). Only used for Bruker input.

Spectral ProcessingChimeric search, RT prediction, misc.


## docs/configuration/spectra

## Spectral Processing – Nextra

Source: https://sage-docs.vercel.app/docs/configuration/spectra

DocumentationConfiguring SageSpectral Processing# Spectral Processing

- min_peaks : Integer. Only process MS2 spectra with at least N peaks (default: 15).
- max_peaks : Integer. Take the top N most intense MS2 peaks to search (default: 150).
- min_matched_peaks : Integer. The minimum number of matched b+y ions required for scoring and reporting PSMs (default: 4).
- max_fragment_charge : Integer. The maximum fragment ion charge states to consider (default: null - use precursor z-1).

Sage does not perform any transformations on spectral intensity.

If deisotope is turned on, Sage will attempt to deisotope and deconvolute the charge state of fragment ions in an MS2 spectra.
This can speed up searches on high-res MS/MS data.

max_fragment_charge , by default, will search each fragment several times against the fragment index with different charge states (up to precursor_charge - 1 ) to try and find candidate peptides - at the cost of increased run time.
If deisotope is turned off, it is recommended to set max_fragment_charge = 1 .

min_matched_peaks should be tuned per-dataset, and should be increased if chimeric searching is enabled

Recommended settings for high-res MS/MS

high res MS/MS```
{

    
"deisotope"
:
 
true
,

    
"min_peaks"
:
 
15
,

    
"max_peaks"
:
 
150
,

    
"min_matched_peaks"
:
 
4
,

    
"max_fragment_charge"
:
 
1
,

}
```

Recommended settings for low-res MS/MS

low res MS/MS```
{

    
"deisotope"
:
 
false
,

    
"min_peaks"
:
 
15
,

    
"max_peaks"
:
 
150
,

    
"min_matched_peaks"
:
 
4
,

    
"max_fragment_charge"
:
 
2
,

}
```

Search TolerancesQuantification


## docs/configuration/tolerance

## Search Tolerances – Nextra

Source: https://sage-docs.vercel.app/docs/configuration/tolerance

DocumentationConfiguring SageSearch Tolerances# Search Tolerances

Several parameters can impact precursor and fragment tolerances during the database search phase.

```
{

    
"precursor_tol"
:
 {

        
"da"
:
 [

            
-500
,

            
100

        ]

    }
,

    
"fragment_tol"
:
 {

        
"ppm"
:
 [

            
-10
,

            
10

        ]

    }
,

    
"isotope_errors"
:
 [
0
,
 
3
]
,

    
"wide_window"
:
 
false

}
```

### Precursor Tolerance

This parameters specifices whether an absolute ( "da" ) or relative ( "ppm" ) tolerance is used for selecting candidate matches.
Sage applies the "left" side of the tolerance to the experimental mass - this means that typical open-searches (i.e looking for a positive delta mass arising from a chemical modification)
will want to use a large negative value here. Using the above configuration, an MS2 spectrum with a 2700 Da precursor mass would search for candidate peptides with a mass between 2200 and 2800 Da.

A standard "closed search" might use the following configuration:

closed search```
{

    
"precursor_tol"
:
 { 
"ppm"
:
 [
-20
,
 
20
] }

}
```

## Fragment Tolerance

Similar to above, you can select whether to use absolute or relative tolerances:

For high-res MS/MS:

High res MS/MS```
{

    
"fragment_tol"
:
 { 
"ppm"
:
 [
-10
,
 
10
] }

}
```

Or for low-res MS/MS:

Low res MS/MS```
{

    
"fragment_tol"
:
 { 
"da"
:
 [
-0.4
,
 
0.4
] }

}
```

## Precursor Charge Range

If precursor charge states are not annotated in the mzML, or if the wide_window setting is turned on, Sage will attempt to search the given precursor m/z at multiple charge states (by default, z=2, z=3, and z=4). This setting can be configured to override this behavior.

Default value :

```
{

  
"precursor_charge"
:
 [
2
,
 
4
]

}
```

## Isotope Errors

This parameter essentially runs a multi-notch or mass offset search for each integer value in the ( left .. right ) range by multipyling the integer value by the mass difference of one C13 neutron.
This can account for incorrect assignment or sequening of the monoisotopic peak.

Default value :

```
{

    
"isotope_errors"
:
 [
0
,
 
0
]

}
```

Typical values :

```
{

    
"isotope_errors"
:
 [
0
,
 
3
]

}
```

## Wide-window mode

💡Setting wide_window will override precursor_tol !

This parameter instructs Sage to dynamically change the precursor tolerance for each spectra based on the isolation window encoded in the mzML file.
This is useful for searching wide-window acquisition (WWA) data, parallel-reaction monitoring (PRM), or data-independent acquisition (DIA) data.
You probably also want to update the following settings as well if you are running a WWA/PRM/DIA search:

```
{

    
"wide_window"
:
 
true
,

    
"chimera"
:
 
true
,

    
// Try running multiple searches with different `report_psms` values

    
"report_psms"
:
 
5

}
```

Fragment Index ConstructionSpectral Processing


## docs/how_it_works

## How Sage Works – Nextra

Source: https://sage-docs.vercel.app/docs/how_it_works

DocumentationHow Sage Works# How Sage Works

Sage is somewhat of a rejection of the UNIX/traditional bioinformatics philosophy of "write programs that do one thing and do it well". (Or, perhaps it
is an expansion of this concept... where "one thing" means "the whole analysis")

While this philosophy works very well for most pipelines, it is at odds with developing high-performance, end-to-end-testable software -
different tools frequently have their own (often-poorly-designed) file formats, requiring time spent serializing and deserializing data into
exotic shapes, and wasted CPU cycles on disk IO operations.

Sage, instead, reads data from storage once and keeps all data in main-memory until it has finished running. The different modules within Sage
(searching, quantification, retention time alignment, peptide-spectrum-match rescoring) all share a unified in-memory database. This eliminates the need
to write intermediate result files, and when paired with the ability to directly stream data from cloud-storage, completely eliminates the need to use a local disk.

Reduction of storage IO and elimination of intermediate results unlocks substantial performance gains. Further performance comes from leveraging Rust's memory model,
and the excellent Rayon library for work-stealing parallelization. Sage is designed such that every operation that can be parallelized is parallelized: reading multiple files in parallel, searching, quantification,
rescoring, and statistical control.

A fragment indexing strategy, as popularized by MSFragger, facilitates fast searches with narrow or wide precursor tolerance.

### Overview of Key Steps and Algorithms

One important thing to note about Sage is that any files that are searched together will undergo global RT-alignment and global FDR control.

Thus, you should only search files together if it makes sense for them to undergo RT-alignment and FDR control together.

#### Generate Fragment Index

Every potential fragment ion from all peptides in the FASTA database is generated and stored in an internal "fragment index". This data structure
enables fast lookups ("what peptides could produce this observed MS2 ion?") with arbitrary precision

#### Load Files

Sage will load many mzML files in parallel - gzipped mzMLs are also supported out-of-the-box. Spectra from these files will undergo spectral preprocessing in parallel
(selection of N most intense peaks, deisotoping)

#### Search Files

Sage searches spectra in parallel. For each MS2 spectrum, Sage finds the list of candidate peptides that have the most experimental-theoretical matches.

```
# Psuedo-code

def
 
score_spectrum
(
spectrum
,
 
n
):

  scoring_table 
=
 
{}

  
for
 ion 
in
 spectrum
:

    
for
 peptide 
in
 fragment_index
.
query
(ion, tolerances):

      scoring_table
[
peptide
]
 
+=
 
1

  
return
 
n_best
(scoring_table, n)
```

Each of these candidate peptides then undergoes "full" scoring: calculation of hyperscore , delta_hyperscore , longest_y_series , etc. The candidate peptide(s)
with the highest hyperscore will be reported. These scores are used to train a machine learning model that will discriminate between target and decoy matches.

#### Retention Time Alignment & Prediction

After all files have been searched, Sage globally aligns their retention times by matching confident PSMs across files. Post-alignment, a linear model is trained to
predict retention times based on peptide sequence. The difference between predicted and observed RT is used as an additional feature for machine learning

#### Machine Learning for PSM rescoring

Sage uses linear discriminant analysis for rescoring peptide-spectrum matches. This algorithm learns the linear combination of a set of features (PSM rank, hyperscore, RT difference vs predicted, etc) that best discriminates
between target and decoy matches. The linear combination is used to produce a single "discriminant score" for each peptide-spectrum match.

To support open-searches, Sage builds a model that predicts the likelihood of a given delta mass shift belonging to a decoy hit. This is used as an additional feature for linear discriminant analysis.

#### Non-parametric modeling of posterior errors

Kernel-density estimation is used to model the distribution of target and decoy discriminant scores. Posterior error probabilities
for PSMs, peptides, and protein groups are then calculated from the model, which are used to calculate q-values and control the false discovery rate.
Picked-peptide and picked-protein approaches are used to maximize discoveries in large searches.

Additionally, Sage calculates a conservative version of FDR: q_value = ( 1 + n_decoys) / n_targets

#### Quantification

##### TMT-based

Sage will quantify and report all reporter ion intensities, regardless of the FDR of the matched peptide assignment. TMT reporter ion intensities are 1:1 with those
reported by ProteomeDiscoverer - Sage can also report signal/noise measurements. More information on configuring TMT searches

##### Label-free

Sage includes a wicked-fast and accurate label-free quantification module that uses direct ion current extraction ( a la FlashLFQ, IonStar) to quantify peptides.
Only confidently-identified peptides will be quantified.

#### Tracing

For each peptide quantified, Sage generates a decoy peptide to control the false-MS1-integration-rate. Both the target and decoy peptides are added to an indexed data structure, analogous to the
fragment index.

For each MS1 spectrum in the dataset, MS1 ion intensities are assigned to all potential peptides (target & decoy) within a fixed-size mass and retention time tolerance, considering all charge
states and isotopologues.

#### Integration

MS1 intensities assigned to each peptide are arranged on a Cartesian grid (think, heatmap) indexed by retention time, isotopologue, and file. Peptide-specific time warping is then performed to align MS1 traces across files, maximizing overlapping signals.

Each grid is then scored according to normalized spectral angle, intensity, and retention time delta from the most-confident RT for that peptide.

#### Output

By default, Sage will write several files to the output_directory configuration option (or, the current directory if not set).

Check out the results documentation when you're ready to interpret!

output_directory- results.json
- results.sage.tsv
- tmt.tsv
- lfq.tsv

Getting Started with SageConfiguring Sage


## docs/index

## Introduction – Nextra

Source: https://sage-docs.vercel.app/docs

DocumentationIntroduction# Introduction

Sage is, at it's core, a proteomics database search engine -
a tool that transforms raw mass spectra from proteomics experiments into peptide identifications
via database searching & spectral matching.

However, Sage includes a variety of advanced features that make it a one-stop shop: retention time prediction, quantification (both isobaric & LFQ), peptide-spectrum match rescoring, and FDR control. You can directly use results from Sage without needing to use other tools for these tasks.

Additionally, Sage was designed with cloud computing in mind - massively parallel processing and the ability to directly stream compressed mass spectrometry data to/from AWS S3 enables unprecedented search speeds with minimal cost.

Sage also runs just as well reading local files from your Mac/PC/Linux device!

### Why use Sage instead of other tools?

Sage is simple to configure , powerful and flexible .
It also happens to be well-tested, mind-boggingly fast , open-source (MIT-licensed) and free.

### Features

- Incredible performance out of the box
- Effortlessly cross-platform (Linux/MacOS/Windows), effortlessly parallel (uses all of your CPU cores)
- Fragment indexing strategy allows for blazing fast narrow and open searches (> 500 Da precursor tolerance)
- Isobaric quantification (MS2/MS3-TMT, or custom reporter ions)
- Label-free quantification : consider all charge states & isotopologues a la FlashLFQ
- Capable of searching for chimeric/co-fragmenting spectra
- Wide-window (dynamic precursor tolerance) search mode - enables WWA/PRM/DIA searches
- Retention time prediction models fit to each LC/MS run
- PSM rescoring using built-in linear discriminant analysis (LDA)
- PEP calculation using a non-parametric model (KDE)
- FDR calculation using target-decoy competition and picked-peptide & picked-protein approaches
- Percolator/Mokapot compatible output
- Configuration by JSON file
- Built-in support for reading gzipped-mzML files
- Support for reading/writing directly from AWS S3

Getting Started with Sage


## docs/results

## Interpreting Sage Results – Nextra

Source: https://sage-docs.vercel.app/docs/results

DocumentationInterpreting Sage Results# Interpreting Sage Results

Sage results are primarily meant to be analyzed using standard programming/data-science tools ( Pandas (opens in a new tab) , Polars (opens in a new tab) , data.table (opens in a new tab) , Rstudio (opens in a new tab) , etc), and I highly encourage you to learn enough of one
of these tools to filter, merge, pivot, and groupby. It is a truly invaluable skill to have for anyone who needs to analyze proteomics datasets (or data in general!). That being said, TSV files are provided
such that end-users can perform some filtering and analyzes in Excel.

with_polars.py```
import
 polars 
as
 pl

## Sum TMT intensitities for all confident matches

df 
=
 (

    pl
.
read_csv
(
"results.sage.tsv"
, separator
=
"\t"
)

    
.
filter
((pl.
col
(
"peptide_q"
) 
<=
 
0.01
) 
&
 (pl.
col
(
"protein_q"
) 
<=
 
0.01
) 
&
 (pl.
col
(
"label"
) 
==
 
1
))

    
.
join
(pl.
read_csv
(
"tmt.tsv"
, separator
=
'\t'
), on
=
[
"filename"
, 
"scannr"
])

    
.
groupby
([
"filename"
, 
"peptide"
, 
"proteins"
]).
agg
(pl.
col
(
"tmt*"
).
sum
())

)
```

## Tab-Separated Format

By default, Sage will write several files to the output_directory configuration option (or, the current directory if not set):

output_directory- results.json
- results.sage.tsv
- results.sage.pin
- tmt.tsv
- lfq.tsv

- results.json contains a record of the search parameters and files used for the analysis. It is possible reproduce the analysis from this file:

reproduce```
sage
 
results.json
```

- results.sage.tsv contains all PSMs (including decoys and non-confident matches) identified in the analysis.
- results.sage.pin is a percolator/Mokapot compatible file, produced if --write-pin is passed as a command-line argument.
- tmt.tsv contains all reporter ions quantified
- lfq.tsv contains MS1 intensities for each peptide quantified

### Parquet File Format

Parquet is a file format for data analysis and distributed queries. Parquet files are natively compressed, and yield smaller file sizes and faster query times than TSV files.
Parquet files can be directly read by Pandas, Polars, and more!

Sage will write parquet files instead of tsv files if you pass --parquet as a command-line argument.

⚠️Serializing results to parquet files is still experimental, and the column names are liable to change!

Notable differences from the standard output are that TMT reporter ion intensities are pre-merged onto
search results, and can be found in the reporter_ion_intensity column. The other columns are largely the same as the TSV format

output_directory- results.json
- results.sage.parquet
- lfq.parquet

#### Results schema

results.sage.parquet```
message schema {

    required byte_array filename (utf8);

    required byte_array scannr (utf8);

    required byte_array peptide (utf8);

    required byte_array stripped_peptide (utf8);

    required byte_array proteins (utf8);

    required int32 num_proteins;

    required int32 rank;

    required boolean is_decoy;

    required float expmass;

    required float calcmass;

    required int32 charge;

    required int32 peptide_len;

    required int32 missed_cleavages;

    required float isotope_error;

    required float precursor_ppm;

    required float fragment_ppm;

    required float hyperscore;

    required float delta_next;

    required float delta_best;

    required float rt;

    required float aligned_rt;

    required float predicted_rt;

    required float delta_rt_model;

    required int32 matched_peaks;

    required int32 longest_b;

    required int32 longest_y;

    required float longest_y_pct;

    required float matched_intensity_pct;

    required int32 scored_candidates;

    required float poisson;

    required float sage_discriminant_score;

    required float posterior_error;

    required float spectrum_q;

    required float peptide_q;

    required float protein_q;

    optional group reporter_ion_intensity (LIST) {

        repeated group list {

            optional float element;

        }

    }

}
```

### LFQ Schema

Notably, the lfq output in parquet format is in long-form

lfq.parquet```
message schema {

    required byte_array peptide (utf8);

    required byte_array stripped_peptide (utf8);

    required byte_array proteins (utf8);

    required boolean is_decoy;

    required float q_value;

    required byte_array filename (utf8);

    required float intensity;

}
```

Bruker File ProcessingSearch Results


## docs/results/lfq

## Label-free Quantification – Nextra

Source: https://sage-docs.vercel.app/docs/results/lfq

DocumentationInterpreting Sage ResultsLabel-free Quantification# Label-free Quantification Results

The lfq.tsv file produced by Sage has the following columns - note that there will be 1 column per file analyzed (wide-format):

| Header | Description |
| --- | --- |
| peptide | Peptide sequence, in ProForma format |
| proteins | Semicolon-delimited list of proteins matching the peptide sequence. |
| q_value | MS1-integration specific q-value |
| score | Score used for q-value calculation |
| spectral_angle | Normalized spectral contrast angle between experimental and theoretical isotopic envelope for this peptide |
| filename_1 | MS1 intensity for peptide in this file |
| filename_N | MS1 intensity for peptide in this file |

Search ResultsTMT Quantification


## docs/results/search

## Search Results – Nextra

Source: https://sage-docs.vercel.app/docs/results/search

DocumentationInterpreting Sage ResultsSearch Results# Search Results

Search results can be found in the results.sage.tsv file. The table below describes the columns that are present in this file.

| Header | Description |
| --- | --- |
| peptide | Peptide sequence, including modifications in ProForma format (e.g., NC[+57.021]HKGSFK). |
| proteins | Semicolon-delimited list of proteins matching the peptide sequence. |
| num_proteins | Number of proteins matching the peptide sequence. |
| filename | File containing this PSM |
| scannr | Spectrum identifier from mzML file. |
| rank | Rank of the PSM. If report_psms > 1 , then the best match will have rank = 1, the second best match will have rank = 2, etc. |
| label | Target/Decoy label (-1: decoy, 1: target). |
| expmass | Experimental mass of the peptide. |
| calcmass | Calculated mass of the peptide. |
| charge | Reported precursor charge. |
| pepide_len | Length of the peptide sequence. |
| missed_cleavages | Number of missed cleavages. |
| isotope_error | C13 isotope error. |
| precursor_ppm | Difference between experimental mass and calculated mass, reported in parts-per-million. |
| fragment_ppm | Average parts-per-million (delta mass) for matched fragment ions compared to theoretical ions. |
| hyperscore | X!Tandem hyperscore for the PSM. |
| delta_next | Difference between the hyperscore of this candidate and the next best candidate. |
| delta_bext | Difference between the hyperscore of the best candidate (rank=1) and this candidate. |
| rt | Retention time in minutes. |
| aligned_rt | Globally aligned retention time. |
| predicted_rt | Predicted retention time, if enabled. |
| delta_rt_model | Difference between predicted and observed retention time. |
| ion_mobility | Ion mobility of the precursor, if present in mzML. |
| predicted_mobility | Predicted ion mobility of the precursor, if ion mobility measurements are present in the mzML. |
| delta_mobility | Difference between predicted and observed ion mobility. |
| matched_peaks | Number of matched theoretical fragment ions. |
| longest_b | Longest b-ion series. |
| longest_y | Longest y-ion series. |
| longest_y_pct | Longest y-ion series, divided by peptide length (as a fraction). |
| matched_intensity_pct | Fraction of MS2 intensity explained by matched b- and y-ions (as a percentage of total MS2 intensity for this spectrum). |
| scored_candidates | Number of scored candidates for this spectrum. |
| poisson | Probability of matching exactly N peaks across all scored candidates (Pr(x=k)). |
| sage_discriminant_score | Combined score from linear discriminant analysis, used for FDR (False Discovery Rate) calculation. |
| posterior_error | Posterior error probability for this PSM / local FDR. |
| spectrum_q | Assigned spectrum-level q-value. |
| peptide_q | Assigned peptide-level q-value. |
| protein_q | Assigned protein-level q-value. |
| ms2_intensity | Total intensity of MS2 spectrum. |

Interpreting Sage ResultsLabel-free Quantification


## docs/results/tmt

## TMT Quantification – Nextra

Source: https://sage-docs.vercel.app/docs/results/tmt

DocumentationInterpreting Sage ResultsTMT Quantification# TMT Quantification Results

The tmt.tsv file produced by Sage has the following columns - each spectrum will be on a separate line:

| Header | Description |
| --- | --- |
| filename | File containing this spectrum |
| scannr | Spectrum identifier from mzML file |
| ion_injection_time | Ion injection time from the mzML specturm |
| tmt_1 | Reporter ion intensity for the first channel |
| tmt_2 | Reporter ion intensity for the second channel |
| tmt_N | Reporter ion intensity for the Nth channel |

Label-free Quantification


## docs/started

## Getting Started – Nextra

Source: https://sage-docs.vercel.app/docs/started

DocumentationGetting Started with Sage# Getting Started

#### Convert your files to mzML

Sage aims for full compatibility with the mzML specification, and has been extensively tested on files from the following two conversion tools:

1. msConvert (opens in a new tab)

- Enable peak-picking/centroiding for MS1 spectra
- Do not use numpress

1. ThermoRawFileParser (opens in a new tab)

- Sage can be used to calculate TMT S/N measurements if ThermoRawFileParser is configured to write noise values into the mzML

Sage can directly read gzipped-mzMLs, as well as mzMLs with internal zlib compression of m/z and intensity values. I generally recommend writing 32-bit, compressed values to mzMLs.

#### Download or install the latest version of Sage

There are several easy ways to download Sage - you don't need to install any additional software, packages, or runtimes to use it:

1. Download the latest binary release (recommended)
2. Compile from source code
3. Install via bioconda
4. Run via Docker

##### Download the latest binary release

This is the easiest way to run Sage - Sage uses a continuous integration/deployment system to automatically compile binaries and publically distribute them as Github Releases (opens in a new tab) .

Visit this link and download the file corresponding to your operating system and CPU architecture.

Most users will probably want one of the following:

| Configuration | Binary |
| --- | --- |
| Mac, Apple Silicon | aarch64-apple-darwin |
| Mac, Intel | x86_64-apple-darwin |
| Windows | x86_64-pc-windows-msvc |
| Linux | x86_64-unknown-linux-gnu |

Additional binary builds are supplied for other, less-common configurations

##### Compile from source code

- Install the Rust programming language compiler (opens in a new tab) toolchain

Once you have Rust installed, you can copy and paste the following lines of code into your terminal (assuming you have git installed!)

```
git
 
clone
 
https://github.com/lazear/sage.git

cd
 
sage

cargo
 
run
 
--release
 
tests/config.json
```

##### Install via bioconda

Sage can be installed from bioconda (opens in a new tab) :

```
conda
 
install
 
-c
 
bioconda
 
-c
 
conda-forge
 
sage-proteomics

sage
 
--help
```

##### Run via Docker

```
docker
 
pull
 
ghcr.io/lazear/sage:latest

docker
 
run
 
-it
 
--rm
 
-v
 ${PWD}
:/data
 
ghcr.io/lazear/sage:latest
 
/app/sage
 
-o
 
/data
 
/data/config.json
```

-v ${PWD}:/data will mount your current directory as /data in the docker image. Make sure all the paths in your command and configuration
use the location in the image and not your local directory

#### Run Sage

Please see the configuration section for details. Understanding how Sage works might also be useful!

Once you're ready to go, run Sage via the command line:

```
# Everything can be configured from a single file

sage
 
experiment_242.json

 

# Or you can set some arguments from the command line!

sage
 
base_config.json
 
-f
 
human.fasta
 
--write-pin
 
*.mzML
```

#### Interpret results

Please see the results section for details!

IntroductionHow Sage Works


## README

## Sage Docs Markdown Export

Exported 17 pages from https://sage-docs.vercel.app/docs
