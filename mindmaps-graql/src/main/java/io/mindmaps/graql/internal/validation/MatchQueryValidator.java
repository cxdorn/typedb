package io.mindmaps.graql.internal.validation;

import io.mindmaps.core.dao.MindmapsTransaction;
import io.mindmaps.graql.api.query.MatchQuery;
import io.mindmaps.graql.api.query.Var;

import java.util.ArrayList;
import java.util.Collection;
import java.util.List;
import java.util.stream.Stream;

import static java.util.stream.Collectors.toList;

/**
 * A validator for a MatchQuery
 */
public class MatchQueryValidator implements Validator {

    private final MatchQuery.Admin matchQuery;

    /**
     * @param matchQuery the match query to validate
     */
    public MatchQueryValidator(MatchQuery.Admin matchQuery) {
        this.matchQuery = matchQuery;
    }

    @Override
    public Stream<String> getErrors(MindmapsTransaction transaction) {
        List<String> errors = new ArrayList<>();

        new PatternValidator(matchQuery.getPattern()).getErrors(transaction).forEach(errors::add);

        Collection<String> patternNames = matchQuery.getPattern().getVars().stream()
                .flatMap(v -> v.getInnerVars().stream()).map(Var.Admin::getName)
                .collect(toList());

        // Find any missing names
        Collection<String> missingNames = new ArrayList<>(matchQuery.getSelectedNames());
        missingNames.removeAll(patternNames);

        missingNames.forEach(missingName -> errors.add(ErrorMessage.SELECT_VAR_NOT_IN_MATCH.getMessage(missingName)));

        return errors.stream();
    }
}
