module Brainfrack

typealias Program String

function get_mappings(program::Program)
    mappings = Dict{}()

    for (index, char) in enumerate(program)
        if char == '['
            mappings[index] = 1
        end
    end

    mappings
end

end
