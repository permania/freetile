fn cool_layout() {
    split(
	v(),
	0.5,
	window(),
	split(
	    h(),
	    0.75,
	    window(),    
	    drain(
		v(),
		no_limit()
	    )
	)
    );
}

cool_layout()