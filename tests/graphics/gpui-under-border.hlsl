    // The later border draw already applies the same outer coverage.
    // Precompensate only the background, preserving background/child/border order.
    if (quad.border_style == 2u) {
        float outer_coverage = saturate(antialias_threshold - outer_sdf);
        float border_coverage = saturate(antialias_threshold - inner_sdf) * input.border_color.a;
        float denominator = 1.0 - outer_coverage * border_coverage;
        float background_coverage = denominator > 0.0
            ? outer_coverage * (1.0 - border_coverage) / denominator
            : 0.0;
        background_color.a *= background_coverage;
        // Coverage is complete here; the ordinary return must not multiply it again.
        return background_color;
    }
