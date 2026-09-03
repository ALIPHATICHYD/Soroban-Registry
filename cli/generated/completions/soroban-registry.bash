_soroban-registry() {
    local i cur prev opts cmd
    COMPREPLY=()
    if [[ "${BASH_VERSINFO[0]}" -ge 4 ]]; then
        cur="$2"
    else
        cur="${COMP_WORDS[COMP_CWORD]}"
    fi
    prev="$3"
    cmd=""
    opts=""

    for i in "${COMP_WORDS[@]:0:COMP_CWORD}"
    do
        case "${cmd},${i}" in
            ",$1")
                cmd="soroban__registry"
                ;;
            soroban__registry,analytics)
                cmd="soroban__registry__subcmd__analytics"
                ;;
            soroban__registry,analyze)
                cmd="soroban__registry__subcmd__analyze"
                ;;
            soroban__registry,api-key)
                cmd="soroban__registry__subcmd__api__subcmd__key"
                ;;
            soroban__registry,audit)
                cmd="soroban__registry__subcmd__audit"
                ;;
            soroban__registry,auth)
                cmd="soroban__registry__subcmd__auth"
                ;;
            soroban__registry,backup)
                cmd="soroban__registry__subcmd__backup"
                ;;
            soroban__registry,batch)
                cmd="soroban__registry__subcmd__batch"
                ;;
            soroban__registry,batch-audit)
                cmd="soroban__registry__subcmd__batch__subcmd__audit"
                ;;
            soroban__registry,batch-deploy)
                cmd="soroban__registry__subcmd__batch__subcmd__deploy"
                ;;
            soroban__registry,batch-export)
                cmd="soroban__registry__subcmd__batch__subcmd__export"
                ;;
            soroban__registry,batch-import)
                cmd="soroban__registry__subcmd__batch__subcmd__import"
                ;;
            soroban__registry,batch-register)
                cmd="soroban__registry__subcmd__batch__subcmd__register"
                ;;
            soroban__registry,batch-update)
                cmd="soroban__registry__subcmd__batch__subcmd__update"
                ;;
            soroban__registry,batch-verify)
                cmd="soroban__registry__subcmd__batch__subcmd__verify"
                ;;
            soroban__registry,breaking-changes)
                cmd="soroban__registry__subcmd__breaking__subcmd__changes"
                ;;
            soroban__registry,cache)
                cmd="soroban__registry__subcmd__cache"
                ;;
            soroban__registry,cicd)
                cmd="soroban__registry__subcmd__cicd"
                ;;
            soroban__registry,compare)
                cmd="soroban__registry__subcmd__compare"
                ;;
            soroban__registry,completion)
                cmd="soroban__registry__subcmd__completion"
                ;;
            soroban__registry,config)
                cmd="soroban__registry__subcmd__config"
                ;;
            soroban__registry,contract)
                cmd="soroban__registry__subcmd__contract"
                ;;
            soroban__registry,coverage)
                cmd="soroban__registry__subcmd__coverage"
                ;;
            soroban__registry,dashboard)
                cmd="soroban__registry__subcmd__dashboard"
                ;;
            soroban__registry,deploy)
                cmd="soroban__registry__subcmd__deploy"
                ;;
            soroban__registry,doc)
                cmd="soroban__registry__subcmd__doc"
                ;;
            soroban__registry,env)
                cmd="soroban__registry__subcmd__env"
                ;;
            soroban__registry,export)
                cmd="soroban__registry__subcmd__export"
                ;;
            soroban__registry,fuzz)
                cmd="soroban__registry__subcmd__fuzz"
                ;;
            soroban__registry,generate-artifacts)
                cmd="soroban__registry__subcmd__generate__subcmd__artifacts"
                ;;
            soroban__registry,help)
                cmd="soroban__registry__subcmd__help"
                ;;
            soroban__registry,history)
                cmd="soroban__registry__subcmd__history"
                ;;
            soroban__registry,import)
                cmd="soroban__registry__subcmd__import"
                ;;
            soroban__registry,incident)
                cmd="soroban__registry__subcmd__incident"
                ;;
            soroban__registry,info)
                cmd="soroban__registry__subcmd__info"
                ;;
            soroban__registry,keys)
                cmd="soroban__registry__subcmd__keys"
                ;;
            soroban__registry,list)
                cmd="soroban__registry__subcmd__list"
                ;;
            soroban__registry,migrate)
                cmd="soroban__registry__subcmd__migrate"
                ;;
            soroban__registry,multisig)
                cmd="soroban__registry__subcmd__multisig"
                ;;
            soroban__registry,network)
                cmd="soroban__registry__subcmd__network"
                ;;
            soroban__registry,openapi)
                cmd="soroban__registry__subcmd__openapi"
                ;;
            soroban__registry,patch)
                cmd="soroban__registry__subcmd__patch"
                ;;
            soroban__registry,perf)
                cmd="soroban__registry__subcmd__perf"
                ;;
            soroban__registry,plugins)
                cmd="soroban__registry__subcmd__plugins"
                ;;
            soroban__registry,policy)
                cmd="soroban__registry__subcmd__policy"
                ;;
            soroban__registry,profile)
                cmd="soroban__registry__subcmd__profile"
                ;;
            soroban__registry,publish)
                cmd="soroban__registry__subcmd__publish"
                ;;
            soroban__registry,publisher)
                cmd="soroban__registry__subcmd__publisher"
                ;;
            soroban__registry,release-notes)
                cmd="soroban__registry__subcmd__release__subcmd__notes"
                ;;
            soroban__registry,repl)
                cmd="soroban__registry__subcmd__repl"
                ;;
            soroban__registry,scan-deps)
                cmd="soroban__registry__subcmd__scan__subcmd__deps"
                ;;
            soroban__registry,search)
                cmd="soroban__registry__subcmd__search"
                ;;
            soroban__registry,sign)
                cmd="soroban__registry__subcmd__sign"
                ;;
            soroban__registry,sla)
                cmd="soroban__registry__subcmd__sla"
                ;;
            soroban__registry,snapshot)
                cmd="soroban__registry__subcmd__snapshot"
                ;;
            soroban__registry,state)
                cmd="soroban__registry__subcmd__state"
                ;;
            soroban__registry,stats)
                cmd="soroban__registry__subcmd__stats"
                ;;
            soroban__registry,test)
                cmd="soroban__registry__subcmd__test"
                ;;
            soroban__registry,track-deployment)
                cmd="soroban__registry__subcmd__track__subcmd__deployment"
                ;;
            soroban__registry,upgrade)
                cmd="soroban__registry__subcmd__upgrade"
                ;;
            soroban__registry,upgrade-analyze)
                cmd="soroban__registry__subcmd__upgrade__subcmd__analyze"
                ;;
            soroban__registry,verify)
                cmd="soroban__registry__subcmd__verify"
                ;;
            soroban__registry,verify-contract)
                cmd="soroban__registry__subcmd__verify__subcmd__contract"
                ;;
            soroban__registry,verify-formal)
                cmd="soroban__registry__subcmd__verify__subcmd__formal"
                ;;
            soroban__registry,verify-package)
                cmd="soroban__registry__subcmd__verify__subcmd__package"
                ;;
            soroban__registry,version)
                cmd="soroban__registry__subcmd__version"
                ;;
            soroban__registry,versions)
                cmd="soroban__registry__subcmd__versions"
                ;;
            soroban__registry,webhook)
                cmd="soroban__registry__subcmd__webhook"
                ;;
            soroban__registry,wizard)
                cmd="soroban__registry__subcmd__wizard"
                ;;
            soroban__registry__subcmd__api__subcmd__key,create)
                cmd="soroban__registry__subcmd__api__subcmd__key__subcmd__create"
                ;;
            soroban__registry__subcmd__api__subcmd__key,delete)
                cmd="soroban__registry__subcmd__api__subcmd__key__subcmd__delete"
                ;;
            soroban__registry__subcmd__api__subcmd__key,help)
                cmd="soroban__registry__subcmd__api__subcmd__key__subcmd__help"
                ;;
            soroban__registry__subcmd__api__subcmd__key,list)
                cmd="soroban__registry__subcmd__api__subcmd__key__subcmd__list"
                ;;
            soroban__registry__subcmd__api__subcmd__key,revoke)
                cmd="soroban__registry__subcmd__api__subcmd__key__subcmd__revoke"
                ;;
            soroban__registry__subcmd__api__subcmd__key__subcmd__help,create)
                cmd="soroban__registry__subcmd__api__subcmd__key__subcmd__help__subcmd__create"
                ;;
            soroban__registry__subcmd__api__subcmd__key__subcmd__help,delete)
                cmd="soroban__registry__subcmd__api__subcmd__key__subcmd__help__subcmd__delete"
                ;;
            soroban__registry__subcmd__api__subcmd__key__subcmd__help,help)
                cmd="soroban__registry__subcmd__api__subcmd__key__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__api__subcmd__key__subcmd__help,list)
                cmd="soroban__registry__subcmd__api__subcmd__key__subcmd__help__subcmd__list"
                ;;
            soroban__registry__subcmd__api__subcmd__key__subcmd__help,revoke)
                cmd="soroban__registry__subcmd__api__subcmd__key__subcmd__help__subcmd__revoke"
                ;;
            soroban__registry__subcmd__auth,help)
                cmd="soroban__registry__subcmd__auth__subcmd__help"
                ;;
            soroban__registry__subcmd__auth,login)
                cmd="soroban__registry__subcmd__auth__subcmd__login"
                ;;
            soroban__registry__subcmd__auth,logout)
                cmd="soroban__registry__subcmd__auth__subcmd__logout"
                ;;
            soroban__registry__subcmd__auth,status)
                cmd="soroban__registry__subcmd__auth__subcmd__status"
                ;;
            soroban__registry__subcmd__auth,token)
                cmd="soroban__registry__subcmd__auth__subcmd__token"
                ;;
            soroban__registry__subcmd__auth__subcmd__help,help)
                cmd="soroban__registry__subcmd__auth__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__auth__subcmd__help,login)
                cmd="soroban__registry__subcmd__auth__subcmd__help__subcmd__login"
                ;;
            soroban__registry__subcmd__auth__subcmd__help,logout)
                cmd="soroban__registry__subcmd__auth__subcmd__help__subcmd__logout"
                ;;
            soroban__registry__subcmd__auth__subcmd__help,status)
                cmd="soroban__registry__subcmd__auth__subcmd__help__subcmd__status"
                ;;
            soroban__registry__subcmd__auth__subcmd__help,token)
                cmd="soroban__registry__subcmd__auth__subcmd__help__subcmd__token"
                ;;
            soroban__registry__subcmd__backup,create)
                cmd="soroban__registry__subcmd__backup__subcmd__create"
                ;;
            soroban__registry__subcmd__backup,help)
                cmd="soroban__registry__subcmd__backup__subcmd__help"
                ;;
            soroban__registry__subcmd__backup,list)
                cmd="soroban__registry__subcmd__backup__subcmd__list"
                ;;
            soroban__registry__subcmd__backup,restore)
                cmd="soroban__registry__subcmd__backup__subcmd__restore"
                ;;
            soroban__registry__subcmd__backup,stats)
                cmd="soroban__registry__subcmd__backup__subcmd__stats"
                ;;
            soroban__registry__subcmd__backup,verify)
                cmd="soroban__registry__subcmd__backup__subcmd__verify"
                ;;
            soroban__registry__subcmd__backup__subcmd__help,create)
                cmd="soroban__registry__subcmd__backup__subcmd__help__subcmd__create"
                ;;
            soroban__registry__subcmd__backup__subcmd__help,help)
                cmd="soroban__registry__subcmd__backup__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__backup__subcmd__help,list)
                cmd="soroban__registry__subcmd__backup__subcmd__help__subcmd__list"
                ;;
            soroban__registry__subcmd__backup__subcmd__help,restore)
                cmd="soroban__registry__subcmd__backup__subcmd__help__subcmd__restore"
                ;;
            soroban__registry__subcmd__backup__subcmd__help,stats)
                cmd="soroban__registry__subcmd__backup__subcmd__help__subcmd__stats"
                ;;
            soroban__registry__subcmd__backup__subcmd__help,verify)
                cmd="soroban__registry__subcmd__backup__subcmd__help__subcmd__verify"
                ;;
            soroban__registry__subcmd__cache,clear)
                cmd="soroban__registry__subcmd__cache__subcmd__clear"
                ;;
            soroban__registry__subcmd__cache,configure)
                cmd="soroban__registry__subcmd__cache__subcmd__configure"
                ;;
            soroban__registry__subcmd__cache,export)
                cmd="soroban__registry__subcmd__cache__subcmd__export"
                ;;
            soroban__registry__subcmd__cache,help)
                cmd="soroban__registry__subcmd__cache__subcmd__help"
                ;;
            soroban__registry__subcmd__cache,optimize)
                cmd="soroban__registry__subcmd__cache__subcmd__optimize"
                ;;
            soroban__registry__subcmd__cache,status)
                cmd="soroban__registry__subcmd__cache__subcmd__status"
                ;;
            soroban__registry__subcmd__cache__subcmd__help,clear)
                cmd="soroban__registry__subcmd__cache__subcmd__help__subcmd__clear"
                ;;
            soroban__registry__subcmd__cache__subcmd__help,configure)
                cmd="soroban__registry__subcmd__cache__subcmd__help__subcmd__configure"
                ;;
            soroban__registry__subcmd__cache__subcmd__help,export)
                cmd="soroban__registry__subcmd__cache__subcmd__help__subcmd__export"
                ;;
            soroban__registry__subcmd__cache__subcmd__help,help)
                cmd="soroban__registry__subcmd__cache__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__cache__subcmd__help,optimize)
                cmd="soroban__registry__subcmd__cache__subcmd__help__subcmd__optimize"
                ;;
            soroban__registry__subcmd__cache__subcmd__help,status)
                cmd="soroban__registry__subcmd__cache__subcmd__help__subcmd__status"
                ;;
            soroban__registry__subcmd__cicd,help)
                cmd="soroban__registry__subcmd__cicd__subcmd__help"
                ;;
            soroban__registry__subcmd__cicd,run)
                cmd="soroban__registry__subcmd__cicd__subcmd__run"
                ;;
            soroban__registry__subcmd__cicd,validate)
                cmd="soroban__registry__subcmd__cicd__subcmd__validate"
                ;;
            soroban__registry__subcmd__cicd__subcmd__help,help)
                cmd="soroban__registry__subcmd__cicd__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__cicd__subcmd__help,run)
                cmd="soroban__registry__subcmd__cicd__subcmd__help__subcmd__run"
                ;;
            soroban__registry__subcmd__cicd__subcmd__help,validate)
                cmd="soroban__registry__subcmd__cicd__subcmd__help__subcmd__validate"
                ;;
            soroban__registry__subcmd__config,contract-get)
                cmd="soroban__registry__subcmd__config__subcmd__contract__subcmd__get"
                ;;
            soroban__registry__subcmd__config,contract-history)
                cmd="soroban__registry__subcmd__config__subcmd__contract__subcmd__history"
                ;;
            soroban__registry__subcmd__config,contract-rollback)
                cmd="soroban__registry__subcmd__config__subcmd__contract__subcmd__rollback"
                ;;
            soroban__registry__subcmd__config,contract-set)
                cmd="soroban__registry__subcmd__config__subcmd__contract__subcmd__set"
                ;;
            soroban__registry__subcmd__config,get)
                cmd="soroban__registry__subcmd__config__subcmd__get"
                ;;
            soroban__registry__subcmd__config,help)
                cmd="soroban__registry__subcmd__config__subcmd__help"
                ;;
            soroban__registry__subcmd__config,list)
                cmd="soroban__registry__subcmd__config__subcmd__list"
                ;;
            soroban__registry__subcmd__config,reset)
                cmd="soroban__registry__subcmd__config__subcmd__reset"
                ;;
            soroban__registry__subcmd__config,set)
                cmd="soroban__registry__subcmd__config__subcmd__set"
                ;;
            soroban__registry__subcmd__config__subcmd__help,contract-get)
                cmd="soroban__registry__subcmd__config__subcmd__help__subcmd__contract__subcmd__get"
                ;;
            soroban__registry__subcmd__config__subcmd__help,contract-history)
                cmd="soroban__registry__subcmd__config__subcmd__help__subcmd__contract__subcmd__history"
                ;;
            soroban__registry__subcmd__config__subcmd__help,contract-rollback)
                cmd="soroban__registry__subcmd__config__subcmd__help__subcmd__contract__subcmd__rollback"
                ;;
            soroban__registry__subcmd__config__subcmd__help,contract-set)
                cmd="soroban__registry__subcmd__config__subcmd__help__subcmd__contract__subcmd__set"
                ;;
            soroban__registry__subcmd__config__subcmd__help,get)
                cmd="soroban__registry__subcmd__config__subcmd__help__subcmd__get"
                ;;
            soroban__registry__subcmd__config__subcmd__help,help)
                cmd="soroban__registry__subcmd__config__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__config__subcmd__help,list)
                cmd="soroban__registry__subcmd__config__subcmd__help__subcmd__list"
                ;;
            soroban__registry__subcmd__config__subcmd__help,reset)
                cmd="soroban__registry__subcmd__config__subcmd__help__subcmd__reset"
                ;;
            soroban__registry__subcmd__config__subcmd__help,set)
                cmd="soroban__registry__subcmd__config__subcmd__help__subcmd__set"
                ;;
            soroban__registry__subcmd__contract,audit)
                cmd="soroban__registry__subcmd__contract__subcmd__audit"
                ;;
            soroban__registry__subcmd__contract,category)
                cmd="soroban__registry__subcmd__contract__subcmd__category"
                ;;
            soroban__registry__subcmd__contract,compatibility)
                cmd="soroban__registry__subcmd__contract__subcmd__compatibility"
                ;;
            soroban__registry__subcmd__contract,dependencies)
                cmd="soroban__registry__subcmd__contract__subcmd__dependencies"
                ;;
            soroban__registry__subcmd__contract,dependency)
                cmd="soroban__registry__subcmd__contract__subcmd__dependency"
                ;;
            soroban__registry__subcmd__contract,dependency-risk)
                cmd="soroban__registry__subcmd__contract__subcmd__dependency__subcmd__risk"
                ;;
            soroban__registry__subcmd__contract,dependents)
                cmd="soroban__registry__subcmd__contract__subcmd__dependents"
                ;;
            soroban__registry__subcmd__contract,deploy)
                cmd="soroban__registry__subcmd__contract__subcmd__deploy"
                ;;
            soroban__registry__subcmd__contract,deprecate)
                cmd="soroban__registry__subcmd__contract__subcmd__deprecate"
                ;;
            soroban__registry__subcmd__contract,details)
                cmd="soroban__registry__subcmd__contract__subcmd__details"
                ;;
            soroban__registry__subcmd__contract,export)
                cmd="soroban__registry__subcmd__contract__subcmd__export"
                ;;
            soroban__registry__subcmd__contract,help)
                cmd="soroban__registry__subcmd__contract__subcmd__help"
                ;;
            soroban__registry__subcmd__contract,highlight)
                cmd="soroban__registry__subcmd__contract__subcmd__highlight"
                ;;
            soroban__registry__subcmd__contract,import)
                cmd="soroban__registry__subcmd__contract__subcmd__import"
                ;;
            soroban__registry__subcmd__contract,interaction)
                cmd="soroban__registry__subcmd__contract__subcmd__interaction"
                ;;
            soroban__registry__subcmd__contract,interfaces)
                cmd="soroban__registry__subcmd__contract__subcmd__interfaces"
                ;;
            soroban__registry__subcmd__contract,list)
                cmd="soroban__registry__subcmd__contract__subcmd__list"
                ;;
            soroban__registry__subcmd__contract,notification)
                cmd="soroban__registry__subcmd__contract__subcmd__notification"
                ;;
            soroban__registry__subcmd__contract,provenance)
                cmd="soroban__registry__subcmd__contract__subcmd__provenance"
                ;;
            soroban__registry__subcmd__contract,register)
                cmd="soroban__registry__subcmd__contract__subcmd__register"
                ;;
            soroban__registry__subcmd__contract,risk)
                cmd="soroban__registry__subcmd__contract__subcmd__risk"
                ;;
            soroban__registry__subcmd__contract,rollback)
                cmd="soroban__registry__subcmd__contract__subcmd__rollback"
                ;;
            soroban__registry__subcmd__contract,search)
                cmd="soroban__registry__subcmd__contract__subcmd__search"
                ;;
            soroban__registry__subcmd__contract,snapshot)
                cmd="soroban__registry__subcmd__contract__subcmd__snapshot"
                ;;
            soroban__registry__subcmd__contract,stats)
                cmd="soroban__registry__subcmd__contract__subcmd__stats"
                ;;
            soroban__registry__subcmd__contract,update)
                cmd="soroban__registry__subcmd__contract__subcmd__update"
                ;;
            soroban__registry__subcmd__contract,verify)
                cmd="soroban__registry__subcmd__contract__subcmd__verify"
                ;;
            soroban__registry__subcmd__contract,verify-build)
                cmd="soroban__registry__subcmd__contract__subcmd__verify__subcmd__build"
                ;;
            soroban__registry__subcmd__contract,verify-snapshot)
                cmd="soroban__registry__subcmd__contract__subcmd__verify__subcmd__snapshot"
                ;;
            soroban__registry__subcmd__contract__subcmd__category,help)
                cmd="soroban__registry__subcmd__contract__subcmd__category__subcmd__help"
                ;;
            soroban__registry__subcmd__contract__subcmd__category,list)
                cmd="soroban__registry__subcmd__contract__subcmd__category__subcmd__list"
                ;;
            soroban__registry__subcmd__contract__subcmd__category,stats)
                cmd="soroban__registry__subcmd__contract__subcmd__category__subcmd__stats"
                ;;
            soroban__registry__subcmd__contract__subcmd__category__subcmd__help,help)
                cmd="soroban__registry__subcmd__contract__subcmd__category__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__contract__subcmd__category__subcmd__help,list)
                cmd="soroban__registry__subcmd__contract__subcmd__category__subcmd__help__subcmd__list"
                ;;
            soroban__registry__subcmd__contract__subcmd__category__subcmd__help,stats)
                cmd="soroban__registry__subcmd__contract__subcmd__category__subcmd__help__subcmd__stats"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,audit)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__audit"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,category)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__category"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,compatibility)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__compatibility"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,dependencies)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__dependencies"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,dependency)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__dependency"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,dependency-risk)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__dependency__subcmd__risk"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,dependents)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__dependents"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,deploy)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__deploy"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,deprecate)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__deprecate"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,details)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__details"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,export)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__export"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,help)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,highlight)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__highlight"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,import)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__import"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,interaction)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__interaction"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,interfaces)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__interfaces"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,list)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__list"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,notification)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__notification"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,provenance)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__provenance"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,register)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__register"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,risk)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__risk"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,rollback)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__rollback"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,search)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__search"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,snapshot)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__snapshot"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,stats)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__stats"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,update)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__update"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,verify)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__verify"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,verify-build)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__verify__subcmd__build"
                ;;
            soroban__registry__subcmd__contract__subcmd__help,verify-snapshot)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__verify__subcmd__snapshot"
                ;;
            soroban__registry__subcmd__contract__subcmd__help__subcmd__category,list)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__category__subcmd__list"
                ;;
            soroban__registry__subcmd__contract__subcmd__help__subcmd__category,stats)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__category__subcmd__stats"
                ;;
            soroban__registry__subcmd__contract__subcmd__help__subcmd__notification,configure)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__notification__subcmd__configure"
                ;;
            soroban__registry__subcmd__contract__subcmd__help__subcmd__notification,list)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__notification__subcmd__list"
                ;;
            soroban__registry__subcmd__contract__subcmd__help__subcmd__notification,subscribe)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__notification__subcmd__subscribe"
                ;;
            soroban__registry__subcmd__contract__subcmd__help__subcmd__notification,test)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__notification__subcmd__test"
                ;;
            soroban__registry__subcmd__contract__subcmd__help__subcmd__notification,unsubscribe)
                cmd="soroban__registry__subcmd__contract__subcmd__help__subcmd__notification__subcmd__unsubscribe"
                ;;
            soroban__registry__subcmd__contract__subcmd__notification,configure)
                cmd="soroban__registry__subcmd__contract__subcmd__notification__subcmd__configure"
                ;;
            soroban__registry__subcmd__contract__subcmd__notification,help)
                cmd="soroban__registry__subcmd__contract__subcmd__notification__subcmd__help"
                ;;
            soroban__registry__subcmd__contract__subcmd__notification,list)
                cmd="soroban__registry__subcmd__contract__subcmd__notification__subcmd__list"
                ;;
            soroban__registry__subcmd__contract__subcmd__notification,subscribe)
                cmd="soroban__registry__subcmd__contract__subcmd__notification__subcmd__subscribe"
                ;;
            soroban__registry__subcmd__contract__subcmd__notification,test)
                cmd="soroban__registry__subcmd__contract__subcmd__notification__subcmd__test"
                ;;
            soroban__registry__subcmd__contract__subcmd__notification,unsubscribe)
                cmd="soroban__registry__subcmd__contract__subcmd__notification__subcmd__unsubscribe"
                ;;
            soroban__registry__subcmd__contract__subcmd__notification__subcmd__help,configure)
                cmd="soroban__registry__subcmd__contract__subcmd__notification__subcmd__help__subcmd__configure"
                ;;
            soroban__registry__subcmd__contract__subcmd__notification__subcmd__help,help)
                cmd="soroban__registry__subcmd__contract__subcmd__notification__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__contract__subcmd__notification__subcmd__help,list)
                cmd="soroban__registry__subcmd__contract__subcmd__notification__subcmd__help__subcmd__list"
                ;;
            soroban__registry__subcmd__contract__subcmd__notification__subcmd__help,subscribe)
                cmd="soroban__registry__subcmd__contract__subcmd__notification__subcmd__help__subcmd__subscribe"
                ;;
            soroban__registry__subcmd__contract__subcmd__notification__subcmd__help,test)
                cmd="soroban__registry__subcmd__contract__subcmd__notification__subcmd__help__subcmd__test"
                ;;
            soroban__registry__subcmd__contract__subcmd__notification__subcmd__help,unsubscribe)
                cmd="soroban__registry__subcmd__contract__subcmd__notification__subcmd__help__subcmd__unsubscribe"
                ;;
            soroban__registry__subcmd__env,copy)
                cmd="soroban__registry__subcmd__env__subcmd__copy"
                ;;
            soroban__registry__subcmd__env,delete)
                cmd="soroban__registry__subcmd__env__subcmd__delete"
                ;;
            soroban__registry__subcmd__env,export)
                cmd="soroban__registry__subcmd__env__subcmd__export"
                ;;
            soroban__registry__subcmd__env,get)
                cmd="soroban__registry__subcmd__env__subcmd__get"
                ;;
            soroban__registry__subcmd__env,help)
                cmd="soroban__registry__subcmd__env__subcmd__help"
                ;;
            soroban__registry__subcmd__env,list)
                cmd="soroban__registry__subcmd__env__subcmd__list"
                ;;
            soroban__registry__subcmd__env,set)
                cmd="soroban__registry__subcmd__env__subcmd__set"
                ;;
            soroban__registry__subcmd__env,switch)
                cmd="soroban__registry__subcmd__env__subcmd__switch"
                ;;
            soroban__registry__subcmd__env__subcmd__help,copy)
                cmd="soroban__registry__subcmd__env__subcmd__help__subcmd__copy"
                ;;
            soroban__registry__subcmd__env__subcmd__help,delete)
                cmd="soroban__registry__subcmd__env__subcmd__help__subcmd__delete"
                ;;
            soroban__registry__subcmd__env__subcmd__help,export)
                cmd="soroban__registry__subcmd__env__subcmd__help__subcmd__export"
                ;;
            soroban__registry__subcmd__env__subcmd__help,get)
                cmd="soroban__registry__subcmd__env__subcmd__help__subcmd__get"
                ;;
            soroban__registry__subcmd__env__subcmd__help,help)
                cmd="soroban__registry__subcmd__env__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__env__subcmd__help,list)
                cmd="soroban__registry__subcmd__env__subcmd__help__subcmd__list"
                ;;
            soroban__registry__subcmd__env__subcmd__help,set)
                cmd="soroban__registry__subcmd__env__subcmd__help__subcmd__set"
                ;;
            soroban__registry__subcmd__env__subcmd__help,switch)
                cmd="soroban__registry__subcmd__env__subcmd__help__subcmd__switch"
                ;;
            soroban__registry__subcmd__help,analytics)
                cmd="soroban__registry__subcmd__help__subcmd__analytics"
                ;;
            soroban__registry__subcmd__help,analyze)
                cmd="soroban__registry__subcmd__help__subcmd__analyze"
                ;;
            soroban__registry__subcmd__help,api-key)
                cmd="soroban__registry__subcmd__help__subcmd__api__subcmd__key"
                ;;
            soroban__registry__subcmd__help,audit)
                cmd="soroban__registry__subcmd__help__subcmd__audit"
                ;;
            soroban__registry__subcmd__help,auth)
                cmd="soroban__registry__subcmd__help__subcmd__auth"
                ;;
            soroban__registry__subcmd__help,backup)
                cmd="soroban__registry__subcmd__help__subcmd__backup"
                ;;
            soroban__registry__subcmd__help,batch)
                cmd="soroban__registry__subcmd__help__subcmd__batch"
                ;;
            soroban__registry__subcmd__help,batch-audit)
                cmd="soroban__registry__subcmd__help__subcmd__batch__subcmd__audit"
                ;;
            soroban__registry__subcmd__help,batch-deploy)
                cmd="soroban__registry__subcmd__help__subcmd__batch__subcmd__deploy"
                ;;
            soroban__registry__subcmd__help,batch-export)
                cmd="soroban__registry__subcmd__help__subcmd__batch__subcmd__export"
                ;;
            soroban__registry__subcmd__help,batch-import)
                cmd="soroban__registry__subcmd__help__subcmd__batch__subcmd__import"
                ;;
            soroban__registry__subcmd__help,batch-register)
                cmd="soroban__registry__subcmd__help__subcmd__batch__subcmd__register"
                ;;
            soroban__registry__subcmd__help,batch-update)
                cmd="soroban__registry__subcmd__help__subcmd__batch__subcmd__update"
                ;;
            soroban__registry__subcmd__help,batch-verify)
                cmd="soroban__registry__subcmd__help__subcmd__batch__subcmd__verify"
                ;;
            soroban__registry__subcmd__help,breaking-changes)
                cmd="soroban__registry__subcmd__help__subcmd__breaking__subcmd__changes"
                ;;
            soroban__registry__subcmd__help,cache)
                cmd="soroban__registry__subcmd__help__subcmd__cache"
                ;;
            soroban__registry__subcmd__help,cicd)
                cmd="soroban__registry__subcmd__help__subcmd__cicd"
                ;;
            soroban__registry__subcmd__help,compare)
                cmd="soroban__registry__subcmd__help__subcmd__compare"
                ;;
            soroban__registry__subcmd__help,completion)
                cmd="soroban__registry__subcmd__help__subcmd__completion"
                ;;
            soroban__registry__subcmd__help,config)
                cmd="soroban__registry__subcmd__help__subcmd__config"
                ;;
            soroban__registry__subcmd__help,contract)
                cmd="soroban__registry__subcmd__help__subcmd__contract"
                ;;
            soroban__registry__subcmd__help,coverage)
                cmd="soroban__registry__subcmd__help__subcmd__coverage"
                ;;
            soroban__registry__subcmd__help,dashboard)
                cmd="soroban__registry__subcmd__help__subcmd__dashboard"
                ;;
            soroban__registry__subcmd__help,deploy)
                cmd="soroban__registry__subcmd__help__subcmd__deploy"
                ;;
            soroban__registry__subcmd__help,doc)
                cmd="soroban__registry__subcmd__help__subcmd__doc"
                ;;
            soroban__registry__subcmd__help,env)
                cmd="soroban__registry__subcmd__help__subcmd__env"
                ;;
            soroban__registry__subcmd__help,export)
                cmd="soroban__registry__subcmd__help__subcmd__export"
                ;;
            soroban__registry__subcmd__help,fuzz)
                cmd="soroban__registry__subcmd__help__subcmd__fuzz"
                ;;
            soroban__registry__subcmd__help,generate-artifacts)
                cmd="soroban__registry__subcmd__help__subcmd__generate__subcmd__artifacts"
                ;;
            soroban__registry__subcmd__help,help)
                cmd="soroban__registry__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__help,history)
                cmd="soroban__registry__subcmd__help__subcmd__history"
                ;;
            soroban__registry__subcmd__help,import)
                cmd="soroban__registry__subcmd__help__subcmd__import"
                ;;
            soroban__registry__subcmd__help,incident)
                cmd="soroban__registry__subcmd__help__subcmd__incident"
                ;;
            soroban__registry__subcmd__help,info)
                cmd="soroban__registry__subcmd__help__subcmd__info"
                ;;
            soroban__registry__subcmd__help,keys)
                cmd="soroban__registry__subcmd__help__subcmd__keys"
                ;;
            soroban__registry__subcmd__help,list)
                cmd="soroban__registry__subcmd__help__subcmd__list"
                ;;
            soroban__registry__subcmd__help,migrate)
                cmd="soroban__registry__subcmd__help__subcmd__migrate"
                ;;
            soroban__registry__subcmd__help,multisig)
                cmd="soroban__registry__subcmd__help__subcmd__multisig"
                ;;
            soroban__registry__subcmd__help,network)
                cmd="soroban__registry__subcmd__help__subcmd__network"
                ;;
            soroban__registry__subcmd__help,openapi)
                cmd="soroban__registry__subcmd__help__subcmd__openapi"
                ;;
            soroban__registry__subcmd__help,patch)
                cmd="soroban__registry__subcmd__help__subcmd__patch"
                ;;
            soroban__registry__subcmd__help,perf)
                cmd="soroban__registry__subcmd__help__subcmd__perf"
                ;;
            soroban__registry__subcmd__help,plugins)
                cmd="soroban__registry__subcmd__help__subcmd__plugins"
                ;;
            soroban__registry__subcmd__help,policy)
                cmd="soroban__registry__subcmd__help__subcmd__policy"
                ;;
            soroban__registry__subcmd__help,profile)
                cmd="soroban__registry__subcmd__help__subcmd__profile"
                ;;
            soroban__registry__subcmd__help,publish)
                cmd="soroban__registry__subcmd__help__subcmd__publish"
                ;;
            soroban__registry__subcmd__help,publisher)
                cmd="soroban__registry__subcmd__help__subcmd__publisher"
                ;;
            soroban__registry__subcmd__help,release-notes)
                cmd="soroban__registry__subcmd__help__subcmd__release__subcmd__notes"
                ;;
            soroban__registry__subcmd__help,repl)
                cmd="soroban__registry__subcmd__help__subcmd__repl"
                ;;
            soroban__registry__subcmd__help,scan-deps)
                cmd="soroban__registry__subcmd__help__subcmd__scan__subcmd__deps"
                ;;
            soroban__registry__subcmd__help,search)
                cmd="soroban__registry__subcmd__help__subcmd__search"
                ;;
            soroban__registry__subcmd__help,sign)
                cmd="soroban__registry__subcmd__help__subcmd__sign"
                ;;
            soroban__registry__subcmd__help,sla)
                cmd="soroban__registry__subcmd__help__subcmd__sla"
                ;;
            soroban__registry__subcmd__help,snapshot)
                cmd="soroban__registry__subcmd__help__subcmd__snapshot"
                ;;
            soroban__registry__subcmd__help,state)
                cmd="soroban__registry__subcmd__help__subcmd__state"
                ;;
            soroban__registry__subcmd__help,stats)
                cmd="soroban__registry__subcmd__help__subcmd__stats"
                ;;
            soroban__registry__subcmd__help,test)
                cmd="soroban__registry__subcmd__help__subcmd__test"
                ;;
            soroban__registry__subcmd__help,track-deployment)
                cmd="soroban__registry__subcmd__help__subcmd__track__subcmd__deployment"
                ;;
            soroban__registry__subcmd__help,upgrade)
                cmd="soroban__registry__subcmd__help__subcmd__upgrade"
                ;;
            soroban__registry__subcmd__help,upgrade-analyze)
                cmd="soroban__registry__subcmd__help__subcmd__upgrade__subcmd__analyze"
                ;;
            soroban__registry__subcmd__help,verify)
                cmd="soroban__registry__subcmd__help__subcmd__verify"
                ;;
            soroban__registry__subcmd__help,verify-contract)
                cmd="soroban__registry__subcmd__help__subcmd__verify__subcmd__contract"
                ;;
            soroban__registry__subcmd__help,verify-formal)
                cmd="soroban__registry__subcmd__help__subcmd__verify__subcmd__formal"
                ;;
            soroban__registry__subcmd__help,verify-package)
                cmd="soroban__registry__subcmd__help__subcmd__verify__subcmd__package"
                ;;
            soroban__registry__subcmd__help,version)
                cmd="soroban__registry__subcmd__help__subcmd__version"
                ;;
            soroban__registry__subcmd__help,versions)
                cmd="soroban__registry__subcmd__help__subcmd__versions"
                ;;
            soroban__registry__subcmd__help,webhook)
                cmd="soroban__registry__subcmd__help__subcmd__webhook"
                ;;
            soroban__registry__subcmd__help,wizard)
                cmd="soroban__registry__subcmd__help__subcmd__wizard"
                ;;
            soroban__registry__subcmd__help__subcmd__api__subcmd__key,create)
                cmd="soroban__registry__subcmd__help__subcmd__api__subcmd__key__subcmd__create"
                ;;
            soroban__registry__subcmd__help__subcmd__api__subcmd__key,delete)
                cmd="soroban__registry__subcmd__help__subcmd__api__subcmd__key__subcmd__delete"
                ;;
            soroban__registry__subcmd__help__subcmd__api__subcmd__key,list)
                cmd="soroban__registry__subcmd__help__subcmd__api__subcmd__key__subcmd__list"
                ;;
            soroban__registry__subcmd__help__subcmd__api__subcmd__key,revoke)
                cmd="soroban__registry__subcmd__help__subcmd__api__subcmd__key__subcmd__revoke"
                ;;
            soroban__registry__subcmd__help__subcmd__auth,login)
                cmd="soroban__registry__subcmd__help__subcmd__auth__subcmd__login"
                ;;
            soroban__registry__subcmd__help__subcmd__auth,logout)
                cmd="soroban__registry__subcmd__help__subcmd__auth__subcmd__logout"
                ;;
            soroban__registry__subcmd__help__subcmd__auth,status)
                cmd="soroban__registry__subcmd__help__subcmd__auth__subcmd__status"
                ;;
            soroban__registry__subcmd__help__subcmd__auth,token)
                cmd="soroban__registry__subcmd__help__subcmd__auth__subcmd__token"
                ;;
            soroban__registry__subcmd__help__subcmd__backup,create)
                cmd="soroban__registry__subcmd__help__subcmd__backup__subcmd__create"
                ;;
            soroban__registry__subcmd__help__subcmd__backup,list)
                cmd="soroban__registry__subcmd__help__subcmd__backup__subcmd__list"
                ;;
            soroban__registry__subcmd__help__subcmd__backup,restore)
                cmd="soroban__registry__subcmd__help__subcmd__backup__subcmd__restore"
                ;;
            soroban__registry__subcmd__help__subcmd__backup,stats)
                cmd="soroban__registry__subcmd__help__subcmd__backup__subcmd__stats"
                ;;
            soroban__registry__subcmd__help__subcmd__backup,verify)
                cmd="soroban__registry__subcmd__help__subcmd__backup__subcmd__verify"
                ;;
            soroban__registry__subcmd__help__subcmd__cache,clear)
                cmd="soroban__registry__subcmd__help__subcmd__cache__subcmd__clear"
                ;;
            soroban__registry__subcmd__help__subcmd__cache,configure)
                cmd="soroban__registry__subcmd__help__subcmd__cache__subcmd__configure"
                ;;
            soroban__registry__subcmd__help__subcmd__cache,export)
                cmd="soroban__registry__subcmd__help__subcmd__cache__subcmd__export"
                ;;
            soroban__registry__subcmd__help__subcmd__cache,optimize)
                cmd="soroban__registry__subcmd__help__subcmd__cache__subcmd__optimize"
                ;;
            soroban__registry__subcmd__help__subcmd__cache,status)
                cmd="soroban__registry__subcmd__help__subcmd__cache__subcmd__status"
                ;;
            soroban__registry__subcmd__help__subcmd__cicd,run)
                cmd="soroban__registry__subcmd__help__subcmd__cicd__subcmd__run"
                ;;
            soroban__registry__subcmd__help__subcmd__cicd,validate)
                cmd="soroban__registry__subcmd__help__subcmd__cicd__subcmd__validate"
                ;;
            soroban__registry__subcmd__help__subcmd__config,contract-get)
                cmd="soroban__registry__subcmd__help__subcmd__config__subcmd__contract__subcmd__get"
                ;;
            soroban__registry__subcmd__help__subcmd__config,contract-history)
                cmd="soroban__registry__subcmd__help__subcmd__config__subcmd__contract__subcmd__history"
                ;;
            soroban__registry__subcmd__help__subcmd__config,contract-rollback)
                cmd="soroban__registry__subcmd__help__subcmd__config__subcmd__contract__subcmd__rollback"
                ;;
            soroban__registry__subcmd__help__subcmd__config,contract-set)
                cmd="soroban__registry__subcmd__help__subcmd__config__subcmd__contract__subcmd__set"
                ;;
            soroban__registry__subcmd__help__subcmd__config,get)
                cmd="soroban__registry__subcmd__help__subcmd__config__subcmd__get"
                ;;
            soroban__registry__subcmd__help__subcmd__config,list)
                cmd="soroban__registry__subcmd__help__subcmd__config__subcmd__list"
                ;;
            soroban__registry__subcmd__help__subcmd__config,reset)
                cmd="soroban__registry__subcmd__help__subcmd__config__subcmd__reset"
                ;;
            soroban__registry__subcmd__help__subcmd__config,set)
                cmd="soroban__registry__subcmd__help__subcmd__config__subcmd__set"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,audit)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__audit"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,category)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__category"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,compatibility)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__compatibility"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,dependencies)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__dependencies"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,dependency)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__dependency"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,dependency-risk)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__dependency__subcmd__risk"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,dependents)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__dependents"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,deploy)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__deploy"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,deprecate)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__deprecate"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,details)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__details"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,export)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__export"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,highlight)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__highlight"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,import)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__import"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,interaction)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__interaction"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,interfaces)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__interfaces"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,list)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__list"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,notification)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__notification"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,provenance)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__provenance"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,register)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__register"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,risk)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__risk"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,rollback)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__rollback"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,search)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__search"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,snapshot)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__snapshot"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,stats)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__stats"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,update)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__update"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,verify)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__verify"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,verify-build)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__verify__subcmd__build"
                ;;
            soroban__registry__subcmd__help__subcmd__contract,verify-snapshot)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__verify__subcmd__snapshot"
                ;;
            soroban__registry__subcmd__help__subcmd__contract__subcmd__category,list)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__category__subcmd__list"
                ;;
            soroban__registry__subcmd__help__subcmd__contract__subcmd__category,stats)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__category__subcmd__stats"
                ;;
            soroban__registry__subcmd__help__subcmd__contract__subcmd__notification,configure)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__notification__subcmd__configure"
                ;;
            soroban__registry__subcmd__help__subcmd__contract__subcmd__notification,list)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__notification__subcmd__list"
                ;;
            soroban__registry__subcmd__help__subcmd__contract__subcmd__notification,subscribe)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__notification__subcmd__subscribe"
                ;;
            soroban__registry__subcmd__help__subcmd__contract__subcmd__notification,test)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__notification__subcmd__test"
                ;;
            soroban__registry__subcmd__help__subcmd__contract__subcmd__notification,unsubscribe)
                cmd="soroban__registry__subcmd__help__subcmd__contract__subcmd__notification__subcmd__unsubscribe"
                ;;
            soroban__registry__subcmd__help__subcmd__env,copy)
                cmd="soroban__registry__subcmd__help__subcmd__env__subcmd__copy"
                ;;
            soroban__registry__subcmd__help__subcmd__env,delete)
                cmd="soroban__registry__subcmd__help__subcmd__env__subcmd__delete"
                ;;
            soroban__registry__subcmd__help__subcmd__env,export)
                cmd="soroban__registry__subcmd__help__subcmd__env__subcmd__export"
                ;;
            soroban__registry__subcmd__help__subcmd__env,get)
                cmd="soroban__registry__subcmd__help__subcmd__env__subcmd__get"
                ;;
            soroban__registry__subcmd__help__subcmd__env,list)
                cmd="soroban__registry__subcmd__help__subcmd__env__subcmd__list"
                ;;
            soroban__registry__subcmd__help__subcmd__env,set)
                cmd="soroban__registry__subcmd__help__subcmd__env__subcmd__set"
                ;;
            soroban__registry__subcmd__help__subcmd__env,switch)
                cmd="soroban__registry__subcmd__help__subcmd__env__subcmd__switch"
                ;;
            soroban__registry__subcmd__help__subcmd__incident,trigger)
                cmd="soroban__registry__subcmd__help__subcmd__incident__subcmd__trigger"
                ;;
            soroban__registry__subcmd__help__subcmd__incident,update)
                cmd="soroban__registry__subcmd__help__subcmd__incident__subcmd__update"
                ;;
            soroban__registry__subcmd__help__subcmd__keys,custody)
                cmd="soroban__registry__subcmd__help__subcmd__keys__subcmd__custody"
                ;;
            soroban__registry__subcmd__help__subcmd__keys,generate)
                cmd="soroban__registry__subcmd__help__subcmd__keys__subcmd__generate"
                ;;
            soroban__registry__subcmd__help__subcmd__keys,log)
                cmd="soroban__registry__subcmd__help__subcmd__keys__subcmd__log"
                ;;
            soroban__registry__subcmd__help__subcmd__keys,revoke)
                cmd="soroban__registry__subcmd__help__subcmd__keys__subcmd__revoke"
                ;;
            soroban__registry__subcmd__help__subcmd__migrate,analyze)
                cmd="soroban__registry__subcmd__help__subcmd__migrate__subcmd__analyze"
                ;;
            soroban__registry__subcmd__help__subcmd__migrate,apply)
                cmd="soroban__registry__subcmd__help__subcmd__migrate__subcmd__apply"
                ;;
            soroban__registry__subcmd__help__subcmd__migrate,generate)
                cmd="soroban__registry__subcmd__help__subcmd__migrate__subcmd__generate"
                ;;
            soroban__registry__subcmd__help__subcmd__migrate,history)
                cmd="soroban__registry__subcmd__help__subcmd__migrate__subcmd__history"
                ;;
            soroban__registry__subcmd__help__subcmd__migrate,preview)
                cmd="soroban__registry__subcmd__help__subcmd__migrate__subcmd__preview"
                ;;
            soroban__registry__subcmd__help__subcmd__migrate,rollback)
                cmd="soroban__registry__subcmd__help__subcmd__migrate__subcmd__rollback"
                ;;
            soroban__registry__subcmd__help__subcmd__migrate,validate)
                cmd="soroban__registry__subcmd__help__subcmd__migrate__subcmd__validate"
                ;;
            soroban__registry__subcmd__help__subcmd__multisig,create-policy)
                cmd="soroban__registry__subcmd__help__subcmd__multisig__subcmd__create__subcmd__policy"
                ;;
            soroban__registry__subcmd__help__subcmd__multisig,create-proposal)
                cmd="soroban__registry__subcmd__help__subcmd__multisig__subcmd__create__subcmd__proposal"
                ;;
            soroban__registry__subcmd__help__subcmd__multisig,execute)
                cmd="soroban__registry__subcmd__help__subcmd__multisig__subcmd__execute"
                ;;
            soroban__registry__subcmd__help__subcmd__multisig,info)
                cmd="soroban__registry__subcmd__help__subcmd__multisig__subcmd__info"
                ;;
            soroban__registry__subcmd__help__subcmd__multisig,list-proposals)
                cmd="soroban__registry__subcmd__help__subcmd__multisig__subcmd__list__subcmd__proposals"
                ;;
            soroban__registry__subcmd__help__subcmd__multisig,sign)
                cmd="soroban__registry__subcmd__help__subcmd__multisig__subcmd__sign"
                ;;
            soroban__registry__subcmd__help__subcmd__network,status)
                cmd="soroban__registry__subcmd__help__subcmd__network__subcmd__status"
                ;;
            soroban__registry__subcmd__help__subcmd__patch,apply)
                cmd="soroban__registry__subcmd__help__subcmd__patch__subcmd__apply"
                ;;
            soroban__registry__subcmd__help__subcmd__patch,create)
                cmd="soroban__registry__subcmd__help__subcmd__patch__subcmd__create"
                ;;
            soroban__registry__subcmd__help__subcmd__patch,deps)
                cmd="soroban__registry__subcmd__help__subcmd__patch__subcmd__deps"
                ;;
            soroban__registry__subcmd__help__subcmd__patch,notify)
                cmd="soroban__registry__subcmd__help__subcmd__patch__subcmd__notify"
                ;;
            soroban__registry__subcmd__help__subcmd__patch__subcmd__deps,list)
                cmd="soroban__registry__subcmd__help__subcmd__patch__subcmd__deps__subcmd__list"
                ;;
            soroban__registry__subcmd__help__subcmd__plugins,config)
                cmd="soroban__registry__subcmd__help__subcmd__plugins__subcmd__config"
                ;;
            soroban__registry__subcmd__help__subcmd__plugins,install)
                cmd="soroban__registry__subcmd__help__subcmd__plugins__subcmd__install"
                ;;
            soroban__registry__subcmd__help__subcmd__plugins,list)
                cmd="soroban__registry__subcmd__help__subcmd__plugins__subcmd__list"
                ;;
            soroban__registry__subcmd__help__subcmd__plugins,marketplace)
                cmd="soroban__registry__subcmd__help__subcmd__plugins__subcmd__marketplace"
                ;;
            soroban__registry__subcmd__help__subcmd__plugins,run)
                cmd="soroban__registry__subcmd__help__subcmd__plugins__subcmd__run"
                ;;
            soroban__registry__subcmd__help__subcmd__plugins,uninstall)
                cmd="soroban__registry__subcmd__help__subcmd__plugins__subcmd__uninstall"
                ;;
            soroban__registry__subcmd__help__subcmd__plugins__subcmd__config,disable)
                cmd="soroban__registry__subcmd__help__subcmd__plugins__subcmd__config__subcmd__disable"
                ;;
            soroban__registry__subcmd__help__subcmd__plugins__subcmd__config,enable)
                cmd="soroban__registry__subcmd__help__subcmd__plugins__subcmd__config__subcmd__enable"
                ;;
            soroban__registry__subcmd__help__subcmd__plugins__subcmd__config,get)
                cmd="soroban__registry__subcmd__help__subcmd__plugins__subcmd__config__subcmd__get"
                ;;
            soroban__registry__subcmd__help__subcmd__plugins__subcmd__config,set)
                cmd="soroban__registry__subcmd__help__subcmd__plugins__subcmd__config__subcmd__set"
                ;;
            soroban__registry__subcmd__help__subcmd__policy,check)
                cmd="soroban__registry__subcmd__help__subcmd__policy__subcmd__check"
                ;;
            soroban__registry__subcmd__help__subcmd__profile,edit)
                cmd="soroban__registry__subcmd__help__subcmd__profile__subcmd__edit"
                ;;
            soroban__registry__subcmd__help__subcmd__profile,export)
                cmd="soroban__registry__subcmd__help__subcmd__profile__subcmd__export"
                ;;
            soroban__registry__subcmd__help__subcmd__profile,list-contracts)
                cmd="soroban__registry__subcmd__help__subcmd__profile__subcmd__list__subcmd__contracts"
                ;;
            soroban__registry__subcmd__help__subcmd__profile,update)
                cmd="soroban__registry__subcmd__help__subcmd__profile__subcmd__update"
                ;;
            soroban__registry__subcmd__help__subcmd__profile,view)
                cmd="soroban__registry__subcmd__help__subcmd__profile__subcmd__view"
                ;;
            soroban__registry__subcmd__help__subcmd__publisher,doctor)
                cmd="soroban__registry__subcmd__help__subcmd__publisher__subcmd__doctor"
                ;;
            soroban__registry__subcmd__help__subcmd__release__subcmd__notes,edit)
                cmd="soroban__registry__subcmd__help__subcmd__release__subcmd__notes__subcmd__edit"
                ;;
            soroban__registry__subcmd__help__subcmd__release__subcmd__notes,generate)
                cmd="soroban__registry__subcmd__help__subcmd__release__subcmd__notes__subcmd__generate"
                ;;
            soroban__registry__subcmd__help__subcmd__release__subcmd__notes,list)
                cmd="soroban__registry__subcmd__help__subcmd__release__subcmd__notes__subcmd__list"
                ;;
            soroban__registry__subcmd__help__subcmd__release__subcmd__notes,publish)
                cmd="soroban__registry__subcmd__help__subcmd__release__subcmd__notes__subcmd__publish"
                ;;
            soroban__registry__subcmd__help__subcmd__release__subcmd__notes,view)
                cmd="soroban__registry__subcmd__help__subcmd__release__subcmd__notes__subcmd__view"
                ;;
            soroban__registry__subcmd__help__subcmd__sla,record)
                cmd="soroban__registry__subcmd__help__subcmd__sla__subcmd__record"
                ;;
            soroban__registry__subcmd__help__subcmd__sla,status)
                cmd="soroban__registry__subcmd__help__subcmd__sla__subcmd__status"
                ;;
            soroban__registry__subcmd__help__subcmd__snapshot,export)
                cmd="soroban__registry__subcmd__help__subcmd__snapshot__subcmd__export"
                ;;
            soroban__registry__subcmd__help__subcmd__snapshot,inspect)
                cmd="soroban__registry__subcmd__help__subcmd__snapshot__subcmd__inspect"
                ;;
            soroban__registry__subcmd__help__subcmd__snapshot,sign)
                cmd="soroban__registry__subcmd__help__subcmd__snapshot__subcmd__sign"
                ;;
            soroban__registry__subcmd__help__subcmd__snapshot,verify)
                cmd="soroban__registry__subcmd__help__subcmd__snapshot__subcmd__verify"
                ;;
            soroban__registry__subcmd__help__subcmd__state,dump)
                cmd="soroban__registry__subcmd__help__subcmd__state__subcmd__dump"
                ;;
            soroban__registry__subcmd__help__subcmd__state,get)
                cmd="soroban__registry__subcmd__help__subcmd__state__subcmd__get"
                ;;
            soroban__registry__subcmd__help__subcmd__state,history)
                cmd="soroban__registry__subcmd__help__subcmd__state__subcmd__history"
                ;;
            soroban__registry__subcmd__help__subcmd__state,set)
                cmd="soroban__registry__subcmd__help__subcmd__state__subcmd__set"
                ;;
            soroban__registry__subcmd__help__subcmd__state,snapshot)
                cmd="soroban__registry__subcmd__help__subcmd__state__subcmd__snapshot"
                ;;
            soroban__registry__subcmd__help__subcmd__state,snapshots)
                cmd="soroban__registry__subcmd__help__subcmd__state__subcmd__snapshots"
                ;;
            soroban__registry__subcmd__help__subcmd__upgrade,analyze)
                cmd="soroban__registry__subcmd__help__subcmd__upgrade__subcmd__analyze"
                ;;
            soroban__registry__subcmd__help__subcmd__upgrade,apply)
                cmd="soroban__registry__subcmd__help__subcmd__upgrade__subcmd__apply"
                ;;
            soroban__registry__subcmd__help__subcmd__upgrade,generate)
                cmd="soroban__registry__subcmd__help__subcmd__upgrade__subcmd__generate"
                ;;
            soroban__registry__subcmd__help__subcmd__upgrade,rollback)
                cmd="soroban__registry__subcmd__help__subcmd__upgrade__subcmd__rollback"
                ;;
            soroban__registry__subcmd__help__subcmd__versions,bump)
                cmd="soroban__registry__subcmd__help__subcmd__versions__subcmd__bump"
                ;;
            soroban__registry__subcmd__help__subcmd__versions,list)
                cmd="soroban__registry__subcmd__help__subcmd__versions__subcmd__list"
                ;;
            soroban__registry__subcmd__help__subcmd__webhook,create)
                cmd="soroban__registry__subcmd__help__subcmd__webhook__subcmd__create"
                ;;
            soroban__registry__subcmd__help__subcmd__webhook,delete)
                cmd="soroban__registry__subcmd__help__subcmd__webhook__subcmd__delete"
                ;;
            soroban__registry__subcmd__help__subcmd__webhook,list)
                cmd="soroban__registry__subcmd__help__subcmd__webhook__subcmd__list"
                ;;
            soroban__registry__subcmd__help__subcmd__webhook,logs)
                cmd="soroban__registry__subcmd__help__subcmd__webhook__subcmd__logs"
                ;;
            soroban__registry__subcmd__help__subcmd__webhook,retry)
                cmd="soroban__registry__subcmd__help__subcmd__webhook__subcmd__retry"
                ;;
            soroban__registry__subcmd__help__subcmd__webhook,test)
                cmd="soroban__registry__subcmd__help__subcmd__webhook__subcmd__test"
                ;;
            soroban__registry__subcmd__help__subcmd__webhook,verify-sig)
                cmd="soroban__registry__subcmd__help__subcmd__webhook__subcmd__verify__subcmd__sig"
                ;;
            soroban__registry__subcmd__incident,help)
                cmd="soroban__registry__subcmd__incident__subcmd__help"
                ;;
            soroban__registry__subcmd__incident,trigger)
                cmd="soroban__registry__subcmd__incident__subcmd__trigger"
                ;;
            soroban__registry__subcmd__incident,update)
                cmd="soroban__registry__subcmd__incident__subcmd__update"
                ;;
            soroban__registry__subcmd__incident__subcmd__help,help)
                cmd="soroban__registry__subcmd__incident__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__incident__subcmd__help,trigger)
                cmd="soroban__registry__subcmd__incident__subcmd__help__subcmd__trigger"
                ;;
            soroban__registry__subcmd__incident__subcmd__help,update)
                cmd="soroban__registry__subcmd__incident__subcmd__help__subcmd__update"
                ;;
            soroban__registry__subcmd__keys,custody)
                cmd="soroban__registry__subcmd__keys__subcmd__custody"
                ;;
            soroban__registry__subcmd__keys,generate)
                cmd="soroban__registry__subcmd__keys__subcmd__generate"
                ;;
            soroban__registry__subcmd__keys,help)
                cmd="soroban__registry__subcmd__keys__subcmd__help"
                ;;
            soroban__registry__subcmd__keys,log)
                cmd="soroban__registry__subcmd__keys__subcmd__log"
                ;;
            soroban__registry__subcmd__keys,revoke)
                cmd="soroban__registry__subcmd__keys__subcmd__revoke"
                ;;
            soroban__registry__subcmd__keys__subcmd__help,custody)
                cmd="soroban__registry__subcmd__keys__subcmd__help__subcmd__custody"
                ;;
            soroban__registry__subcmd__keys__subcmd__help,generate)
                cmd="soroban__registry__subcmd__keys__subcmd__help__subcmd__generate"
                ;;
            soroban__registry__subcmd__keys__subcmd__help,help)
                cmd="soroban__registry__subcmd__keys__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__keys__subcmd__help,log)
                cmd="soroban__registry__subcmd__keys__subcmd__help__subcmd__log"
                ;;
            soroban__registry__subcmd__keys__subcmd__help,revoke)
                cmd="soroban__registry__subcmd__keys__subcmd__help__subcmd__revoke"
                ;;
            soroban__registry__subcmd__migrate,analyze)
                cmd="soroban__registry__subcmd__migrate__subcmd__analyze"
                ;;
            soroban__registry__subcmd__migrate,apply)
                cmd="soroban__registry__subcmd__migrate__subcmd__apply"
                ;;
            soroban__registry__subcmd__migrate,generate)
                cmd="soroban__registry__subcmd__migrate__subcmd__generate"
                ;;
            soroban__registry__subcmd__migrate,help)
                cmd="soroban__registry__subcmd__migrate__subcmd__help"
                ;;
            soroban__registry__subcmd__migrate,history)
                cmd="soroban__registry__subcmd__migrate__subcmd__history"
                ;;
            soroban__registry__subcmd__migrate,preview)
                cmd="soroban__registry__subcmd__migrate__subcmd__preview"
                ;;
            soroban__registry__subcmd__migrate,rollback)
                cmd="soroban__registry__subcmd__migrate__subcmd__rollback"
                ;;
            soroban__registry__subcmd__migrate,validate)
                cmd="soroban__registry__subcmd__migrate__subcmd__validate"
                ;;
            soroban__registry__subcmd__migrate__subcmd__help,analyze)
                cmd="soroban__registry__subcmd__migrate__subcmd__help__subcmd__analyze"
                ;;
            soroban__registry__subcmd__migrate__subcmd__help,apply)
                cmd="soroban__registry__subcmd__migrate__subcmd__help__subcmd__apply"
                ;;
            soroban__registry__subcmd__migrate__subcmd__help,generate)
                cmd="soroban__registry__subcmd__migrate__subcmd__help__subcmd__generate"
                ;;
            soroban__registry__subcmd__migrate__subcmd__help,help)
                cmd="soroban__registry__subcmd__migrate__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__migrate__subcmd__help,history)
                cmd="soroban__registry__subcmd__migrate__subcmd__help__subcmd__history"
                ;;
            soroban__registry__subcmd__migrate__subcmd__help,preview)
                cmd="soroban__registry__subcmd__migrate__subcmd__help__subcmd__preview"
                ;;
            soroban__registry__subcmd__migrate__subcmd__help,rollback)
                cmd="soroban__registry__subcmd__migrate__subcmd__help__subcmd__rollback"
                ;;
            soroban__registry__subcmd__migrate__subcmd__help,validate)
                cmd="soroban__registry__subcmd__migrate__subcmd__help__subcmd__validate"
                ;;
            soroban__registry__subcmd__multisig,create-policy)
                cmd="soroban__registry__subcmd__multisig__subcmd__create__subcmd__policy"
                ;;
            soroban__registry__subcmd__multisig,create-proposal)
                cmd="soroban__registry__subcmd__multisig__subcmd__create__subcmd__proposal"
                ;;
            soroban__registry__subcmd__multisig,execute)
                cmd="soroban__registry__subcmd__multisig__subcmd__execute"
                ;;
            soroban__registry__subcmd__multisig,help)
                cmd="soroban__registry__subcmd__multisig__subcmd__help"
                ;;
            soroban__registry__subcmd__multisig,info)
                cmd="soroban__registry__subcmd__multisig__subcmd__info"
                ;;
            soroban__registry__subcmd__multisig,list-proposals)
                cmd="soroban__registry__subcmd__multisig__subcmd__list__subcmd__proposals"
                ;;
            soroban__registry__subcmd__multisig,sign)
                cmd="soroban__registry__subcmd__multisig__subcmd__sign"
                ;;
            soroban__registry__subcmd__multisig__subcmd__help,create-policy)
                cmd="soroban__registry__subcmd__multisig__subcmd__help__subcmd__create__subcmd__policy"
                ;;
            soroban__registry__subcmd__multisig__subcmd__help,create-proposal)
                cmd="soroban__registry__subcmd__multisig__subcmd__help__subcmd__create__subcmd__proposal"
                ;;
            soroban__registry__subcmd__multisig__subcmd__help,execute)
                cmd="soroban__registry__subcmd__multisig__subcmd__help__subcmd__execute"
                ;;
            soroban__registry__subcmd__multisig__subcmd__help,help)
                cmd="soroban__registry__subcmd__multisig__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__multisig__subcmd__help,info)
                cmd="soroban__registry__subcmd__multisig__subcmd__help__subcmd__info"
                ;;
            soroban__registry__subcmd__multisig__subcmd__help,list-proposals)
                cmd="soroban__registry__subcmd__multisig__subcmd__help__subcmd__list__subcmd__proposals"
                ;;
            soroban__registry__subcmd__multisig__subcmd__help,sign)
                cmd="soroban__registry__subcmd__multisig__subcmd__help__subcmd__sign"
                ;;
            soroban__registry__subcmd__network,help)
                cmd="soroban__registry__subcmd__network__subcmd__help"
                ;;
            soroban__registry__subcmd__network,status)
                cmd="soroban__registry__subcmd__network__subcmd__status"
                ;;
            soroban__registry__subcmd__network__subcmd__help,help)
                cmd="soroban__registry__subcmd__network__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__network__subcmd__help,status)
                cmd="soroban__registry__subcmd__network__subcmd__help__subcmd__status"
                ;;
            soroban__registry__subcmd__patch,apply)
                cmd="soroban__registry__subcmd__patch__subcmd__apply"
                ;;
            soroban__registry__subcmd__patch,create)
                cmd="soroban__registry__subcmd__patch__subcmd__create"
                ;;
            soroban__registry__subcmd__patch,deps)
                cmd="soroban__registry__subcmd__patch__subcmd__deps"
                ;;
            soroban__registry__subcmd__patch,help)
                cmd="soroban__registry__subcmd__patch__subcmd__help"
                ;;
            soroban__registry__subcmd__patch,notify)
                cmd="soroban__registry__subcmd__patch__subcmd__notify"
                ;;
            soroban__registry__subcmd__patch__subcmd__deps,help)
                cmd="soroban__registry__subcmd__patch__subcmd__deps__subcmd__help"
                ;;
            soroban__registry__subcmd__patch__subcmd__deps,list)
                cmd="soroban__registry__subcmd__patch__subcmd__deps__subcmd__list"
                ;;
            soroban__registry__subcmd__patch__subcmd__deps__subcmd__help,help)
                cmd="soroban__registry__subcmd__patch__subcmd__deps__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__patch__subcmd__deps__subcmd__help,list)
                cmd="soroban__registry__subcmd__patch__subcmd__deps__subcmd__help__subcmd__list"
                ;;
            soroban__registry__subcmd__patch__subcmd__help,apply)
                cmd="soroban__registry__subcmd__patch__subcmd__help__subcmd__apply"
                ;;
            soroban__registry__subcmd__patch__subcmd__help,create)
                cmd="soroban__registry__subcmd__patch__subcmd__help__subcmd__create"
                ;;
            soroban__registry__subcmd__patch__subcmd__help,deps)
                cmd="soroban__registry__subcmd__patch__subcmd__help__subcmd__deps"
                ;;
            soroban__registry__subcmd__patch__subcmd__help,help)
                cmd="soroban__registry__subcmd__patch__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__patch__subcmd__help,notify)
                cmd="soroban__registry__subcmd__patch__subcmd__help__subcmd__notify"
                ;;
            soroban__registry__subcmd__patch__subcmd__help__subcmd__deps,list)
                cmd="soroban__registry__subcmd__patch__subcmd__help__subcmd__deps__subcmd__list"
                ;;
            soroban__registry__subcmd__plugins,config)
                cmd="soroban__registry__subcmd__plugins__subcmd__config"
                ;;
            soroban__registry__subcmd__plugins,help)
                cmd="soroban__registry__subcmd__plugins__subcmd__help"
                ;;
            soroban__registry__subcmd__plugins,install)
                cmd="soroban__registry__subcmd__plugins__subcmd__install"
                ;;
            soroban__registry__subcmd__plugins,list)
                cmd="soroban__registry__subcmd__plugins__subcmd__list"
                ;;
            soroban__registry__subcmd__plugins,marketplace)
                cmd="soroban__registry__subcmd__plugins__subcmd__marketplace"
                ;;
            soroban__registry__subcmd__plugins,run)
                cmd="soroban__registry__subcmd__plugins__subcmd__run"
                ;;
            soroban__registry__subcmd__plugins,uninstall)
                cmd="soroban__registry__subcmd__plugins__subcmd__uninstall"
                ;;
            soroban__registry__subcmd__plugins__subcmd__config,disable)
                cmd="soroban__registry__subcmd__plugins__subcmd__config__subcmd__disable"
                ;;
            soroban__registry__subcmd__plugins__subcmd__config,enable)
                cmd="soroban__registry__subcmd__plugins__subcmd__config__subcmd__enable"
                ;;
            soroban__registry__subcmd__plugins__subcmd__config,get)
                cmd="soroban__registry__subcmd__plugins__subcmd__config__subcmd__get"
                ;;
            soroban__registry__subcmd__plugins__subcmd__config,help)
                cmd="soroban__registry__subcmd__plugins__subcmd__config__subcmd__help"
                ;;
            soroban__registry__subcmd__plugins__subcmd__config,set)
                cmd="soroban__registry__subcmd__plugins__subcmd__config__subcmd__set"
                ;;
            soroban__registry__subcmd__plugins__subcmd__config__subcmd__help,disable)
                cmd="soroban__registry__subcmd__plugins__subcmd__config__subcmd__help__subcmd__disable"
                ;;
            soroban__registry__subcmd__plugins__subcmd__config__subcmd__help,enable)
                cmd="soroban__registry__subcmd__plugins__subcmd__config__subcmd__help__subcmd__enable"
                ;;
            soroban__registry__subcmd__plugins__subcmd__config__subcmd__help,get)
                cmd="soroban__registry__subcmd__plugins__subcmd__config__subcmd__help__subcmd__get"
                ;;
            soroban__registry__subcmd__plugins__subcmd__config__subcmd__help,help)
                cmd="soroban__registry__subcmd__plugins__subcmd__config__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__plugins__subcmd__config__subcmd__help,set)
                cmd="soroban__registry__subcmd__plugins__subcmd__config__subcmd__help__subcmd__set"
                ;;
            soroban__registry__subcmd__plugins__subcmd__help,config)
                cmd="soroban__registry__subcmd__plugins__subcmd__help__subcmd__config"
                ;;
            soroban__registry__subcmd__plugins__subcmd__help,help)
                cmd="soroban__registry__subcmd__plugins__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__plugins__subcmd__help,install)
                cmd="soroban__registry__subcmd__plugins__subcmd__help__subcmd__install"
                ;;
            soroban__registry__subcmd__plugins__subcmd__help,list)
                cmd="soroban__registry__subcmd__plugins__subcmd__help__subcmd__list"
                ;;
            soroban__registry__subcmd__plugins__subcmd__help,marketplace)
                cmd="soroban__registry__subcmd__plugins__subcmd__help__subcmd__marketplace"
                ;;
            soroban__registry__subcmd__plugins__subcmd__help,run)
                cmd="soroban__registry__subcmd__plugins__subcmd__help__subcmd__run"
                ;;
            soroban__registry__subcmd__plugins__subcmd__help,uninstall)
                cmd="soroban__registry__subcmd__plugins__subcmd__help__subcmd__uninstall"
                ;;
            soroban__registry__subcmd__plugins__subcmd__help__subcmd__config,disable)
                cmd="soroban__registry__subcmd__plugins__subcmd__help__subcmd__config__subcmd__disable"
                ;;
            soroban__registry__subcmd__plugins__subcmd__help__subcmd__config,enable)
                cmd="soroban__registry__subcmd__plugins__subcmd__help__subcmd__config__subcmd__enable"
                ;;
            soroban__registry__subcmd__plugins__subcmd__help__subcmd__config,get)
                cmd="soroban__registry__subcmd__plugins__subcmd__help__subcmd__config__subcmd__get"
                ;;
            soroban__registry__subcmd__plugins__subcmd__help__subcmd__config,set)
                cmd="soroban__registry__subcmd__plugins__subcmd__help__subcmd__config__subcmd__set"
                ;;
            soroban__registry__subcmd__policy,check)
                cmd="soroban__registry__subcmd__policy__subcmd__check"
                ;;
            soroban__registry__subcmd__policy,help)
                cmd="soroban__registry__subcmd__policy__subcmd__help"
                ;;
            soroban__registry__subcmd__policy__subcmd__help,check)
                cmd="soroban__registry__subcmd__policy__subcmd__help__subcmd__check"
                ;;
            soroban__registry__subcmd__policy__subcmd__help,help)
                cmd="soroban__registry__subcmd__policy__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__profile,edit)
                cmd="soroban__registry__subcmd__profile__subcmd__edit"
                ;;
            soroban__registry__subcmd__profile,export)
                cmd="soroban__registry__subcmd__profile__subcmd__export"
                ;;
            soroban__registry__subcmd__profile,help)
                cmd="soroban__registry__subcmd__profile__subcmd__help"
                ;;
            soroban__registry__subcmd__profile,list-contracts)
                cmd="soroban__registry__subcmd__profile__subcmd__list__subcmd__contracts"
                ;;
            soroban__registry__subcmd__profile,update)
                cmd="soroban__registry__subcmd__profile__subcmd__update"
                ;;
            soroban__registry__subcmd__profile,view)
                cmd="soroban__registry__subcmd__profile__subcmd__view"
                ;;
            soroban__registry__subcmd__profile__subcmd__help,edit)
                cmd="soroban__registry__subcmd__profile__subcmd__help__subcmd__edit"
                ;;
            soroban__registry__subcmd__profile__subcmd__help,export)
                cmd="soroban__registry__subcmd__profile__subcmd__help__subcmd__export"
                ;;
            soroban__registry__subcmd__profile__subcmd__help,help)
                cmd="soroban__registry__subcmd__profile__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__profile__subcmd__help,list-contracts)
                cmd="soroban__registry__subcmd__profile__subcmd__help__subcmd__list__subcmd__contracts"
                ;;
            soroban__registry__subcmd__profile__subcmd__help,update)
                cmd="soroban__registry__subcmd__profile__subcmd__help__subcmd__update"
                ;;
            soroban__registry__subcmd__profile__subcmd__help,view)
                cmd="soroban__registry__subcmd__profile__subcmd__help__subcmd__view"
                ;;
            soroban__registry__subcmd__publisher,doctor)
                cmd="soroban__registry__subcmd__publisher__subcmd__doctor"
                ;;
            soroban__registry__subcmd__publisher,help)
                cmd="soroban__registry__subcmd__publisher__subcmd__help"
                ;;
            soroban__registry__subcmd__publisher__subcmd__help,doctor)
                cmd="soroban__registry__subcmd__publisher__subcmd__help__subcmd__doctor"
                ;;
            soroban__registry__subcmd__publisher__subcmd__help,help)
                cmd="soroban__registry__subcmd__publisher__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__release__subcmd__notes,edit)
                cmd="soroban__registry__subcmd__release__subcmd__notes__subcmd__edit"
                ;;
            soroban__registry__subcmd__release__subcmd__notes,generate)
                cmd="soroban__registry__subcmd__release__subcmd__notes__subcmd__generate"
                ;;
            soroban__registry__subcmd__release__subcmd__notes,help)
                cmd="soroban__registry__subcmd__release__subcmd__notes__subcmd__help"
                ;;
            soroban__registry__subcmd__release__subcmd__notes,list)
                cmd="soroban__registry__subcmd__release__subcmd__notes__subcmd__list"
                ;;
            soroban__registry__subcmd__release__subcmd__notes,publish)
                cmd="soroban__registry__subcmd__release__subcmd__notes__subcmd__publish"
                ;;
            soroban__registry__subcmd__release__subcmd__notes,view)
                cmd="soroban__registry__subcmd__release__subcmd__notes__subcmd__view"
                ;;
            soroban__registry__subcmd__release__subcmd__notes__subcmd__help,edit)
                cmd="soroban__registry__subcmd__release__subcmd__notes__subcmd__help__subcmd__edit"
                ;;
            soroban__registry__subcmd__release__subcmd__notes__subcmd__help,generate)
                cmd="soroban__registry__subcmd__release__subcmd__notes__subcmd__help__subcmd__generate"
                ;;
            soroban__registry__subcmd__release__subcmd__notes__subcmd__help,help)
                cmd="soroban__registry__subcmd__release__subcmd__notes__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__release__subcmd__notes__subcmd__help,list)
                cmd="soroban__registry__subcmd__release__subcmd__notes__subcmd__help__subcmd__list"
                ;;
            soroban__registry__subcmd__release__subcmd__notes__subcmd__help,publish)
                cmd="soroban__registry__subcmd__release__subcmd__notes__subcmd__help__subcmd__publish"
                ;;
            soroban__registry__subcmd__release__subcmd__notes__subcmd__help,view)
                cmd="soroban__registry__subcmd__release__subcmd__notes__subcmd__help__subcmd__view"
                ;;
            soroban__registry__subcmd__sla,help)
                cmd="soroban__registry__subcmd__sla__subcmd__help"
                ;;
            soroban__registry__subcmd__sla,record)
                cmd="soroban__registry__subcmd__sla__subcmd__record"
                ;;
            soroban__registry__subcmd__sla,status)
                cmd="soroban__registry__subcmd__sla__subcmd__status"
                ;;
            soroban__registry__subcmd__sla__subcmd__help,help)
                cmd="soroban__registry__subcmd__sla__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__sla__subcmd__help,record)
                cmd="soroban__registry__subcmd__sla__subcmd__help__subcmd__record"
                ;;
            soroban__registry__subcmd__sla__subcmd__help,status)
                cmd="soroban__registry__subcmd__sla__subcmd__help__subcmd__status"
                ;;
            soroban__registry__subcmd__snapshot,export)
                cmd="soroban__registry__subcmd__snapshot__subcmd__export"
                ;;
            soroban__registry__subcmd__snapshot,help)
                cmd="soroban__registry__subcmd__snapshot__subcmd__help"
                ;;
            soroban__registry__subcmd__snapshot,inspect)
                cmd="soroban__registry__subcmd__snapshot__subcmd__inspect"
                ;;
            soroban__registry__subcmd__snapshot,sign)
                cmd="soroban__registry__subcmd__snapshot__subcmd__sign"
                ;;
            soroban__registry__subcmd__snapshot,verify)
                cmd="soroban__registry__subcmd__snapshot__subcmd__verify"
                ;;
            soroban__registry__subcmd__snapshot__subcmd__help,export)
                cmd="soroban__registry__subcmd__snapshot__subcmd__help__subcmd__export"
                ;;
            soroban__registry__subcmd__snapshot__subcmd__help,help)
                cmd="soroban__registry__subcmd__snapshot__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__snapshot__subcmd__help,inspect)
                cmd="soroban__registry__subcmd__snapshot__subcmd__help__subcmd__inspect"
                ;;
            soroban__registry__subcmd__snapshot__subcmd__help,sign)
                cmd="soroban__registry__subcmd__snapshot__subcmd__help__subcmd__sign"
                ;;
            soroban__registry__subcmd__snapshot__subcmd__help,verify)
                cmd="soroban__registry__subcmd__snapshot__subcmd__help__subcmd__verify"
                ;;
            soroban__registry__subcmd__state,dump)
                cmd="soroban__registry__subcmd__state__subcmd__dump"
                ;;
            soroban__registry__subcmd__state,get)
                cmd="soroban__registry__subcmd__state__subcmd__get"
                ;;
            soroban__registry__subcmd__state,help)
                cmd="soroban__registry__subcmd__state__subcmd__help"
                ;;
            soroban__registry__subcmd__state,history)
                cmd="soroban__registry__subcmd__state__subcmd__history"
                ;;
            soroban__registry__subcmd__state,set)
                cmd="soroban__registry__subcmd__state__subcmd__set"
                ;;
            soroban__registry__subcmd__state,snapshot)
                cmd="soroban__registry__subcmd__state__subcmd__snapshot"
                ;;
            soroban__registry__subcmd__state,snapshots)
                cmd="soroban__registry__subcmd__state__subcmd__snapshots"
                ;;
            soroban__registry__subcmd__state__subcmd__help,dump)
                cmd="soroban__registry__subcmd__state__subcmd__help__subcmd__dump"
                ;;
            soroban__registry__subcmd__state__subcmd__help,get)
                cmd="soroban__registry__subcmd__state__subcmd__help__subcmd__get"
                ;;
            soroban__registry__subcmd__state__subcmd__help,help)
                cmd="soroban__registry__subcmd__state__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__state__subcmd__help,history)
                cmd="soroban__registry__subcmd__state__subcmd__help__subcmd__history"
                ;;
            soroban__registry__subcmd__state__subcmd__help,set)
                cmd="soroban__registry__subcmd__state__subcmd__help__subcmd__set"
                ;;
            soroban__registry__subcmd__state__subcmd__help,snapshot)
                cmd="soroban__registry__subcmd__state__subcmd__help__subcmd__snapshot"
                ;;
            soroban__registry__subcmd__state__subcmd__help,snapshots)
                cmd="soroban__registry__subcmd__state__subcmd__help__subcmd__snapshots"
                ;;
            soroban__registry__subcmd__upgrade,analyze)
                cmd="soroban__registry__subcmd__upgrade__subcmd__analyze"
                ;;
            soroban__registry__subcmd__upgrade,apply)
                cmd="soroban__registry__subcmd__upgrade__subcmd__apply"
                ;;
            soroban__registry__subcmd__upgrade,generate)
                cmd="soroban__registry__subcmd__upgrade__subcmd__generate"
                ;;
            soroban__registry__subcmd__upgrade,help)
                cmd="soroban__registry__subcmd__upgrade__subcmd__help"
                ;;
            soroban__registry__subcmd__upgrade,rollback)
                cmd="soroban__registry__subcmd__upgrade__subcmd__rollback"
                ;;
            soroban__registry__subcmd__upgrade__subcmd__help,analyze)
                cmd="soroban__registry__subcmd__upgrade__subcmd__help__subcmd__analyze"
                ;;
            soroban__registry__subcmd__upgrade__subcmd__help,apply)
                cmd="soroban__registry__subcmd__upgrade__subcmd__help__subcmd__apply"
                ;;
            soroban__registry__subcmd__upgrade__subcmd__help,generate)
                cmd="soroban__registry__subcmd__upgrade__subcmd__help__subcmd__generate"
                ;;
            soroban__registry__subcmd__upgrade__subcmd__help,help)
                cmd="soroban__registry__subcmd__upgrade__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__upgrade__subcmd__help,rollback)
                cmd="soroban__registry__subcmd__upgrade__subcmd__help__subcmd__rollback"
                ;;
            soroban__registry__subcmd__versions,bump)
                cmd="soroban__registry__subcmd__versions__subcmd__bump"
                ;;
            soroban__registry__subcmd__versions,help)
                cmd="soroban__registry__subcmd__versions__subcmd__help"
                ;;
            soroban__registry__subcmd__versions,list)
                cmd="soroban__registry__subcmd__versions__subcmd__list"
                ;;
            soroban__registry__subcmd__versions__subcmd__help,bump)
                cmd="soroban__registry__subcmd__versions__subcmd__help__subcmd__bump"
                ;;
            soroban__registry__subcmd__versions__subcmd__help,help)
                cmd="soroban__registry__subcmd__versions__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__versions__subcmd__help,list)
                cmd="soroban__registry__subcmd__versions__subcmd__help__subcmd__list"
                ;;
            soroban__registry__subcmd__webhook,create)
                cmd="soroban__registry__subcmd__webhook__subcmd__create"
                ;;
            soroban__registry__subcmd__webhook,delete)
                cmd="soroban__registry__subcmd__webhook__subcmd__delete"
                ;;
            soroban__registry__subcmd__webhook,help)
                cmd="soroban__registry__subcmd__webhook__subcmd__help"
                ;;
            soroban__registry__subcmd__webhook,list)
                cmd="soroban__registry__subcmd__webhook__subcmd__list"
                ;;
            soroban__registry__subcmd__webhook,logs)
                cmd="soroban__registry__subcmd__webhook__subcmd__logs"
                ;;
            soroban__registry__subcmd__webhook,retry)
                cmd="soroban__registry__subcmd__webhook__subcmd__retry"
                ;;
            soroban__registry__subcmd__webhook,test)
                cmd="soroban__registry__subcmd__webhook__subcmd__test"
                ;;
            soroban__registry__subcmd__webhook,verify-sig)
                cmd="soroban__registry__subcmd__webhook__subcmd__verify__subcmd__sig"
                ;;
            soroban__registry__subcmd__webhook__subcmd__help,create)
                cmd="soroban__registry__subcmd__webhook__subcmd__help__subcmd__create"
                ;;
            soroban__registry__subcmd__webhook__subcmd__help,delete)
                cmd="soroban__registry__subcmd__webhook__subcmd__help__subcmd__delete"
                ;;
            soroban__registry__subcmd__webhook__subcmd__help,help)
                cmd="soroban__registry__subcmd__webhook__subcmd__help__subcmd__help"
                ;;
            soroban__registry__subcmd__webhook__subcmd__help,list)
                cmd="soroban__registry__subcmd__webhook__subcmd__help__subcmd__list"
                ;;
            soroban__registry__subcmd__webhook__subcmd__help,logs)
                cmd="soroban__registry__subcmd__webhook__subcmd__help__subcmd__logs"
                ;;
            soroban__registry__subcmd__webhook__subcmd__help,retry)
                cmd="soroban__registry__subcmd__webhook__subcmd__help__subcmd__retry"
                ;;
            soroban__registry__subcmd__webhook__subcmd__help,test)
                cmd="soroban__registry__subcmd__webhook__subcmd__help__subcmd__test"
                ;;
            soroban__registry__subcmd__webhook__subcmd__help,verify-sig)
                cmd="soroban__registry__subcmd__webhook__subcmd__help__subcmd__verify__subcmd__sig"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        soroban__registry)
            opts="-v -h -V --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help --version analytics stats publish list info search compare completion generate-artifacts version dashboard breaking-changes migrate upgrade-analyze export import doc openapi deploy versions batch upgrade wizard repl history patch incident multisig fuzz perf profile test audit sla config auth backup state verify-formal scan-deps coverage sign verify-package verify verify-contract keys policy publisher contract api-key batch-verify webhook release-notes cicd network batch-register batch-audit batch-deploy batch-export batch-import batch-update analyze track-deployment plugins cache env snapshot help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__analytics)
            opts="-v -h --period --format --sort --export --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --period)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sort)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --export)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__analyze)
            opts="-o -v -h --network --report-format --output --api-url --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --report-format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -o)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__api__subcmd__key)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help create list delete revoke help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__api__subcmd__key__subcmd__create)
            opts="-v -h --expires --scopes --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --expires)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --scopes)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__api__subcmd__key__subcmd__delete)
            opts="-v -h --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__api__subcmd__key__subcmd__help)
            opts="create list delete revoke help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__api__subcmd__key__subcmd__help__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__api__subcmd__key__subcmd__help__subcmd__delete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__api__subcmd__key__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__api__subcmd__key__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__api__subcmd__key__subcmd__help__subcmd__revoke)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__api__subcmd__key__subcmd__list)
            opts="-v -h --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__api__subcmd__key__subcmd__revoke)
            opts="-v -h --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__audit)
            opts="-o -v -h --format --output --fail-on --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -o)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --fail-on)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__auth)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help login logout status token help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__auth__subcmd__help)
            opts="login logout status token help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__auth__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__auth__subcmd__help__subcmd__login)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__auth__subcmd__help__subcmd__logout)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__auth__subcmd__help__subcmd__status)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__auth__subcmd__help__subcmd__token)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__auth__subcmd__login)
            opts="-v -h --method --identity --secret --scopes --expires --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --method)
                    COMPREPLY=($(compgen -W "github stellar api-key" -- "${cur}"))
                    return 0
                    ;;
                --identity)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --secret)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --scopes)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --expires)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__auth__subcmd__logout)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__auth__subcmd__status)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__auth__subcmd__token)
            opts="-v -h --scopes --expires --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --scopes)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --expires)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__backup)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help create list restore verify stats help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__backup__subcmd__create)
            opts="-v -h --include-state --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__backup__subcmd__help)
            opts="create list restore verify stats help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__backup__subcmd__help__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__backup__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__backup__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__backup__subcmd__help__subcmd__restore)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__backup__subcmd__help__subcmd__stats)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__backup__subcmd__help__subcmd__verify)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__backup__subcmd__list)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__backup__subcmd__restore)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__backup__subcmd__stats)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__backup__subcmd__verify)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__batch)
            opts="-v -h --file --value --rollback-on-error --recipients --message-type --template --preview --schedule --channels --filter --atomic --report --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --value)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --recipients)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --message-type)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --template)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --schedule)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --channels)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --filter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --report)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__batch__subcmd__audit)
            opts="-v -h --format --output-dir --fail-on --high-risk --profile --export --json --api-url --network --timeout --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --fail-on)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --export)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__batch__subcmd__deploy)
            opts="-v -h --networks --signer --atomic --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --networks)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --signer)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__batch__subcmd__export)
            opts="-v -h --filter --format --organize --compress --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --filter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__batch__subcmd__import)
            opts="-v -h --format --on-duplicate --dry-run --atomic --output-dir --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --on-duplicate)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__batch__subcmd__register)
            opts="-v -h --manifest --publisher --dry-run --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --manifest)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --publisher)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__batch__subcmd__update)
            opts="-v -h --file --filter --preview --if --user-id --rollback-on-error --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --filter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --if)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --user-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__batch__subcmd__verify)
            opts="-v -h --file --contracts --network --category --age --initiated-by --level --export --output --schedule --json --api-url --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --contracts)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --category)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --age)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --initiated-by)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --level)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --export)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --schedule)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__breaking__subcmd__changes)
            opts="-v -h --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__cache)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help clear status configure optimize export help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__cache__subcmd__clear)
            opts="-v -h --level --key --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --level)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --key)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__cache__subcmd__configure)
            opts="-v -h --ttl --max-size --compression --auto-refresh --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --ttl)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-size)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --compression)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --auto-refresh)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__cache__subcmd__export)
            opts="-v -h --format --include-stale --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__cache__subcmd__help)
            opts="clear status configure optimize export help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__cache__subcmd__help__subcmd__clear)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__cache__subcmd__help__subcmd__configure)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__cache__subcmd__help__subcmd__export)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__cache__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__cache__subcmd__help__subcmd__optimize)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__cache__subcmd__help__subcmd__status)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__cache__subcmd__optimize)
            opts="-v -h --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__cache__subcmd__status)
            opts="-v -h --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__cicd)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help run validate help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__cicd__subcmd__help)
            opts="run validate help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__cicd__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__cicd__subcmd__help__subcmd__run)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__cicd__subcmd__help__subcmd__validate)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__cicd__subcmd__run)
            opts="-v -h --contract-path --network --skip-scan --auto-register --json --api-url --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --contract-path)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__cicd__subcmd__validate)
            opts="-v -h --contract-path --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --contract-path)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__compare)
            opts="-v -h --json --export --format --exit-code --diff --fields --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --export)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --diff)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --fields)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__completion)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help bash zsh fish elvish power-shell"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__config)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help get set list reset contract-get contract-set contract-history contract-rollback help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__config__subcmd__contract__subcmd__get)
            opts="-v -h --contract-id --environment --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --contract-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --environment)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__config__subcmd__contract__subcmd__history)
            opts="-v -h --contract-id --environment --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --contract-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --environment)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__config__subcmd__contract__subcmd__rollback)
            opts="-v -h --contract-id --environment --version --created-by --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --contract-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --environment)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --version)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --created-by)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__config__subcmd__contract__subcmd__set)
            opts="-v -h --contract-id --environment --config-data --secrets-data --created-by --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --contract-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --environment)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config-data)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --secrets-data)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --created-by)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__config__subcmd__get)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__config__subcmd__help)
            opts="get set list reset contract-get contract-set contract-history contract-rollback help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__config__subcmd__help__subcmd__contract__subcmd__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__config__subcmd__help__subcmd__contract__subcmd__history)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__config__subcmd__help__subcmd__contract__subcmd__rollback)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__config__subcmd__help__subcmd__contract__subcmd__set)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__config__subcmd__help__subcmd__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__config__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__config__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__config__subcmd__help__subcmd__reset)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__config__subcmd__help__subcmd__set)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__config__subcmd__list)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__config__subcmd__reset)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__config__subcmd__set)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help list search snapshot verify-snapshot risk deploy register verify interfaces provenance verify-build compatibility details stats export highlight interaction dependency dependencies dependents dependency-risk category update import rollback audit deprecate notification help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__audit)
            opts="-v -h --lockfile --fix --init --contracts --format --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --lockfile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --contracts)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__category)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help list stats help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__category__subcmd__help)
            opts="list stats help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__category__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__category__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__category__subcmd__help__subcmd__stats)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__category__subcmd__list)
            opts="-v -h --network --format --export --api-url --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --export)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__category__subcmd__stats)
            opts="-v -h --network --format --export --api-url --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --export)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__compatibility)
            opts="-v -h --from --to --from-network-passphrase --to-network-passphrase --strict --fail-on --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --from)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --to)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --from-network-passphrase)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --to-network-passphrase)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --fail-on)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__dependencies)
            opts="-v -h --network --transitive --depth --include-telemetry --json --api-url --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --depth)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__dependency)
            opts="-v -h --depth --format --summary --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --depth)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__dependency__subcmd__risk)
            opts="-v -h --network --depth --fail-on --json --api-url --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --depth)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --fail-on)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__dependents)
            opts="-v -h --network --transitive --depth --include-telemetry --json --api-url --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --depth)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__deploy)
            opts="-v -h --name --description --category --network --icon --interactive --publisher --tags --skip-abi --json --api-url --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --name)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --description)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --category)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --icon)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --publisher)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --tags)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__deprecate)
            opts="-y -v -h --reason --replacement --private-key --migration-guide --grace-period-days --yes --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --reason)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --replacement)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --private-key)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --migration-guide)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --grace-period-days)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__details)
            opts="-v -h --network --json --api-url --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__export)
            opts="-o -f -v -h --output --format --network --category --since --compress --include-related --page-size --api-url --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --output)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -o)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -f)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --category)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --since)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --page-size)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help)
            opts="list search snapshot verify-snapshot risk deploy register verify interfaces provenance verify-build compatibility details stats export highlight interaction dependency dependencies dependents dependency-risk category update import rollback audit deprecate notification help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__audit)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__category)
            opts="list stats"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__category__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__category__subcmd__stats)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__compatibility)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__dependencies)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__dependency)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__dependency__subcmd__risk)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__dependents)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__deploy)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__deprecate)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__details)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__export)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__highlight)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__import)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__interaction)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__interfaces)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__notification)
            opts="subscribe unsubscribe list configure test"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__notification__subcmd__configure)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__notification__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__notification__subcmd__subscribe)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__notification__subcmd__test)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__notification__subcmd__unsubscribe)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__provenance)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__register)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__risk)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__rollback)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__search)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__snapshot)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__stats)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__update)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__verify)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__verify__subcmd__build)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__help__subcmd__verify__subcmd__snapshot)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__highlight)
            opts="-v -h --action --token --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --action)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --token)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__import)
            opts="-f -o -v -h --format --on-duplicate --network-map --dry-run --validate --atomic --report-output --output-dir --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -f)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --on-duplicate)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network-map)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --report-output)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -o)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__interaction)
            opts="-v -h --limit --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --limit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__interfaces)
            opts="-v -h --wasm --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --wasm)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__list)
            opts="-l -o -c -f -v -h --limit --offset --networks --category --format --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --limit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -l)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --offset)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -o)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --networks)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --category)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -c)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -f)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__notification)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help subscribe unsubscribe list configure test help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__notification__subcmd__configure)
            opts="-v -h --alerts --channels --frequency --networks --categories --target --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --alerts)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --channels)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --frequency)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --networks)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --categories)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --target)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__notification__subcmd__help)
            opts="subscribe unsubscribe list configure test help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__notification__subcmd__help__subcmd__configure)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__notification__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__notification__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__notification__subcmd__help__subcmd__subscribe)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__notification__subcmd__help__subcmd__test)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__notification__subcmd__help__subcmd__unsubscribe)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__notification__subcmd__list)
            opts="-v -h --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__notification__subcmd__subscribe)
            opts="-v -h --alerts --channels --frequency --networks --categories --target --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --alerts)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --channels)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --frequency)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --networks)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --categories)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --target)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__notification__subcmd__test)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__notification__subcmd__unsubscribe)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__provenance)
            opts="-v -h --manifest --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --manifest)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__register)
            opts="-v -h --file --batch --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__risk)
            opts="-v -h --network --threshold --json --api-url --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --threshold)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__rollback)
            opts="-y -v -h --reason --private-key --yes --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --reason)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --private-key)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__search)
            opts="-v -h --networks --category --tags --verified-only --limit --offset --cursor --pagination --all --max-items --max-pages --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --networks)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --category)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --tags)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --limit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --offset)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --cursor)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --pagination)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-items)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-pages)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__snapshot)
            opts="-o -v -h --output --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --output)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -o)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__stats)
            opts="-o -v -h --network --category --top-n --format --output --compare --api-url --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --category)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --top-n)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -o)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --compare)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__update)
            opts="-y -v -h --name --description --category --tags --icon --homepage --dry-run --yes --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --name)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --description)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --category)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --tags)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --icon)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --homepage)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__verify)
            opts="-v -h --wasm --network --json --strict --batch --no-cache --api-url --timeout --profile --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --wasm)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__verify__subcmd__build)
            opts="-v -h --manifest --source-dir --expected-hash --allow-toolchain-mismatch --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --manifest)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --source-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --expected-hash)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__contract__subcmd__verify__subcmd__snapshot)
            opts="-v -h --expect-key --max-age-days --fetch-key --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --expect-key)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-age-days)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__coverage)
            opts="-v -h --tests --threshold --output --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --tests)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --threshold)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__dashboard)
            opts="-v -h --refresh-rate --category --ws-url --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --refresh-rate)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --category)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --ws-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__deploy)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__doc)
            opts="-v -h --output --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --output)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__env)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help set get list copy delete export switch help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__env__subcmd__copy)
            opts="-v -h --from --to --overwrite --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --from)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --to)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__env__subcmd__delete)
            opts="-v -h --env --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --env)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__env__subcmd__export)
            opts="-v -h --env --format --merged --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --env)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -W "shell json dotenv" -- "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__env__subcmd__get)
            opts="-v -h --env --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --env)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__env__subcmd__help)
            opts="set get list copy delete export switch help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__env__subcmd__help__subcmd__copy)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__env__subcmd__help__subcmd__delete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__env__subcmd__help__subcmd__export)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__env__subcmd__help__subcmd__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__env__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__env__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__env__subcmd__help__subcmd__set)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__env__subcmd__help__subcmd__switch)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__env__subcmd__list)
            opts="-v -h --env --all --merged --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --env)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__env__subcmd__set)
            opts="-v -h --env --show-value --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --env)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__env__subcmd__switch)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__export)
            opts="-o -f -v -h --id --output --contract-dir --format --filter --page-size --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -o)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --contract-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -f)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --filter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --page-size)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__fuzz)
            opts="-v -h --contract-path --duration --timeout --threads --max-cases --output --minimize --api-url --network --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --contract-path)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --duration)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --threads)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-cases)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__generate__subcmd__artifacts)
            opts="-v -h --check --output-dir --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --output-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help)
            opts="analytics stats publish list info search compare completion generate-artifacts version dashboard breaking-changes migrate upgrade-analyze export import doc openapi deploy versions batch upgrade wizard repl history patch incident multisig fuzz perf profile test audit sla config auth backup state verify-formal scan-deps coverage sign verify-package verify verify-contract keys policy publisher contract api-key batch-verify webhook release-notes cicd network batch-register batch-audit batch-deploy batch-export batch-import batch-update analyze track-deployment plugins cache env snapshot help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__analytics)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__analyze)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__api__subcmd__key)
            opts="create list delete revoke"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__api__subcmd__key__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__api__subcmd__key__subcmd__delete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__api__subcmd__key__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__api__subcmd__key__subcmd__revoke)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__audit)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__auth)
            opts="login logout status token"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__auth__subcmd__login)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__auth__subcmd__logout)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__auth__subcmd__status)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__auth__subcmd__token)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__backup)
            opts="create list restore verify stats"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__backup__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__backup__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__backup__subcmd__restore)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__backup__subcmd__stats)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__backup__subcmd__verify)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__batch)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__batch__subcmd__audit)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__batch__subcmd__deploy)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__batch__subcmd__export)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__batch__subcmd__import)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__batch__subcmd__register)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__batch__subcmd__update)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__batch__subcmd__verify)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__breaking__subcmd__changes)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__cache)
            opts="clear status configure optimize export"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__cache__subcmd__clear)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__cache__subcmd__configure)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__cache__subcmd__export)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__cache__subcmd__optimize)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__cache__subcmd__status)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__cicd)
            opts="run validate"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__cicd__subcmd__run)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__cicd__subcmd__validate)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__compare)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__completion)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__config)
            opts="get set list reset contract-get contract-set contract-history contract-rollback"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__config__subcmd__contract__subcmd__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__config__subcmd__contract__subcmd__history)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__config__subcmd__contract__subcmd__rollback)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__config__subcmd__contract__subcmd__set)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__config__subcmd__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__config__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__config__subcmd__reset)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__config__subcmd__set)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract)
            opts="list search snapshot verify-snapshot risk deploy register verify interfaces provenance verify-build compatibility details stats export highlight interaction dependency dependencies dependents dependency-risk category update import rollback audit deprecate notification"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__audit)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__category)
            opts="list stats"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__category__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__category__subcmd__stats)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__compatibility)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__dependencies)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__dependency)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__dependency__subcmd__risk)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__dependents)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__deploy)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__deprecate)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__details)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__export)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__highlight)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__import)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__interaction)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__interfaces)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__notification)
            opts="subscribe unsubscribe list configure test"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__notification__subcmd__configure)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__notification__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__notification__subcmd__subscribe)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__notification__subcmd__test)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__notification__subcmd__unsubscribe)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__provenance)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__register)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__risk)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__rollback)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__search)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__snapshot)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__stats)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__update)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__verify)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__verify__subcmd__build)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__contract__subcmd__verify__subcmd__snapshot)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__coverage)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__dashboard)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__deploy)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__doc)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__env)
            opts="set get list copy delete export switch"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__env__subcmd__copy)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__env__subcmd__delete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__env__subcmd__export)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__env__subcmd__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__env__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__env__subcmd__set)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__env__subcmd__switch)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__export)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__fuzz)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__generate__subcmd__artifacts)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__history)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__import)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__incident)
            opts="trigger update"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__incident__subcmd__trigger)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__incident__subcmd__update)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__info)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__keys)
            opts="generate revoke custody log"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__keys__subcmd__custody)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__keys__subcmd__generate)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__keys__subcmd__log)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__keys__subcmd__revoke)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__migrate)
            opts="preview analyze generate validate apply rollback history"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__migrate__subcmd__analyze)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__migrate__subcmd__apply)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__migrate__subcmd__generate)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__migrate__subcmd__history)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__migrate__subcmd__preview)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__migrate__subcmd__rollback)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__migrate__subcmd__validate)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__multisig)
            opts="create-policy create-proposal sign execute info list-proposals"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__multisig__subcmd__create__subcmd__policy)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__multisig__subcmd__create__subcmd__proposal)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__multisig__subcmd__execute)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__multisig__subcmd__info)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__multisig__subcmd__list__subcmd__proposals)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__multisig__subcmd__sign)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__network)
            opts="status"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__network__subcmd__status)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__openapi)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__patch)
            opts="create notify apply deps"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__patch__subcmd__apply)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__patch__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__patch__subcmd__deps)
            opts="list"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__patch__subcmd__deps__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__patch__subcmd__notify)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__perf)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__plugins)
            opts="list marketplace install uninstall run config"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__plugins__subcmd__config)
            opts="get set disable enable"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__plugins__subcmd__config__subcmd__disable)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__plugins__subcmd__config__subcmd__enable)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__plugins__subcmd__config__subcmd__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__plugins__subcmd__config__subcmd__set)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__plugins__subcmd__install)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__plugins__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__plugins__subcmd__marketplace)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__plugins__subcmd__run)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__plugins__subcmd__uninstall)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__policy)
            opts="check"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__policy__subcmd__check)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__profile)
            opts="view edit update list-contracts export"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__profile__subcmd__edit)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__profile__subcmd__export)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__profile__subcmd__list__subcmd__contracts)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__profile__subcmd__update)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__profile__subcmd__view)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__publish)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__publisher)
            opts="doctor"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__publisher__subcmd__doctor)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__release__subcmd__notes)
            opts="generate view edit publish list"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__release__subcmd__notes__subcmd__edit)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__release__subcmd__notes__subcmd__generate)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__release__subcmd__notes__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__release__subcmd__notes__subcmd__publish)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__release__subcmd__notes__subcmd__view)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__repl)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__scan__subcmd__deps)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__search)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__sign)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__sla)
            opts="record status"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__sla__subcmd__record)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__sla__subcmd__status)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__snapshot)
            opts="export sign verify inspect"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__snapshot__subcmd__export)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__snapshot__subcmd__inspect)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__snapshot__subcmd__sign)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__snapshot__subcmd__verify)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__state)
            opts="get set dump snapshot snapshots history"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__state__subcmd__dump)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__state__subcmd__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__state__subcmd__history)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__state__subcmd__set)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__state__subcmd__snapshot)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__state__subcmd__snapshots)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__stats)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__test)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__track__subcmd__deployment)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__upgrade)
            opts="analyze apply rollback generate"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__upgrade__subcmd__analyze)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__upgrade__subcmd__analyze)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__upgrade__subcmd__apply)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__upgrade__subcmd__generate)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__upgrade__subcmd__rollback)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__verify)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__verify__subcmd__contract)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__verify__subcmd__formal)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__verify__subcmd__package)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__version)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__versions)
            opts="list bump"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__versions__subcmd__bump)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__versions__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__webhook)
            opts="create list delete test logs retry verify-sig"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__webhook__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__webhook__subcmd__delete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__webhook__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__webhook__subcmd__logs)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__webhook__subcmd__retry)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__webhook__subcmd__test)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__webhook__subcmd__verify__subcmd__sig)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__help__subcmd__wizard)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__history)
            opts="-v -h --search --limit --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --search)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --limit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__import)
            opts="-v -h --format --output-dir --validate --dry-run --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__incident)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help trigger update help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__incident__subcmd__help)
            opts="trigger update help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__incident__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__incident__subcmd__help__subcmd__trigger)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__incident__subcmd__help__subcmd__update)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__incident__subcmd__trigger)
            opts="-v -h --severity --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --severity)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__incident__subcmd__update)
            opts="-v -h --state --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --state)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__info)
            opts="-v -h --json --raw --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__keys)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help generate revoke custody log help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__keys__subcmd__custody)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__keys__subcmd__generate)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__keys__subcmd__help)
            opts="generate revoke custody log help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__keys__subcmd__help__subcmd__custody)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__keys__subcmd__help__subcmd__generate)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__keys__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__keys__subcmd__help__subcmd__log)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__keys__subcmd__help__subcmd__revoke)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__keys__subcmd__log)
            opts="-v -h --contract-id --entry-type --limit --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --contract-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --entry-type)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --limit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__keys__subcmd__revoke)
            opts="-v -h --revoked-by --reason --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --revoked-by)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --reason)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__list)
            opts="-l -o -c -f -v -h --limit --offset --networks --category --format --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --limit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -l)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --offset)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -o)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --networks)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --category)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -c)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -f)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__migrate)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help preview analyze generate validate apply rollback history help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__migrate__subcmd__analyze)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__migrate__subcmd__apply)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__migrate__subcmd__generate)
            opts="-v -h --language --output --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --language)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__migrate__subcmd__help)
            opts="preview analyze generate validate apply rollback history help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__migrate__subcmd__help__subcmd__analyze)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__migrate__subcmd__help__subcmd__apply)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__migrate__subcmd__help__subcmd__generate)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__migrate__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__migrate__subcmd__help__subcmd__history)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__migrate__subcmd__help__subcmd__preview)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__migrate__subcmd__help__subcmd__rollback)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__migrate__subcmd__help__subcmd__validate)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__migrate__subcmd__history)
            opts="-v -h --limit --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --limit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__migrate__subcmd__preview)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__migrate__subcmd__rollback)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__migrate__subcmd__validate)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__multisig)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help create-policy create-proposal sign execute info list-proposals help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__multisig__subcmd__create__subcmd__policy)
            opts="-v -h --name --threshold --signers --expiry-secs --created-by --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --name)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --threshold)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --signers)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --expiry-secs)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --created-by)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__multisig__subcmd__create__subcmd__proposal)
            opts="-v -h --contract-name --contract-id --wasm-hash --network --policy-id --proposer --description --api-url --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --contract-name)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --contract-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --wasm-hash)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --policy-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --proposer)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --description)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__multisig__subcmd__execute)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__multisig__subcmd__help)
            opts="create-policy create-proposal sign execute info list-proposals help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__multisig__subcmd__help__subcmd__create__subcmd__policy)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__multisig__subcmd__help__subcmd__create__subcmd__proposal)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__multisig__subcmd__help__subcmd__execute)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__multisig__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__multisig__subcmd__help__subcmd__info)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__multisig__subcmd__help__subcmd__list__subcmd__proposals)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__multisig__subcmd__help__subcmd__sign)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__multisig__subcmd__info)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__multisig__subcmd__list__subcmd__proposals)
            opts="-v -h --status --limit --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --status)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --limit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__multisig__subcmd__sign)
            opts="-v -h --signer --signature-data --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --signer)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --signature-data)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__network)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help status help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__network__subcmd__help)
            opts="status help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__network__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__network__subcmd__help__subcmd__status)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__network__subcmd__status)
            opts="-v -h --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__openapi)
            opts="-o -f -v -h --output --format --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --output)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -o)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -f)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__patch)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help create notify apply deps help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__patch__subcmd__apply)
            opts="-v -h --contract-id --patch-id --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --contract-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --patch-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__patch__subcmd__create)
            opts="-v -h --version --hash --severity --rollout --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --version)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --hash)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --severity)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --rollout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__patch__subcmd__deps)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help list help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__patch__subcmd__deps__subcmd__help)
            opts="list help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__patch__subcmd__deps__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__patch__subcmd__deps__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__patch__subcmd__deps__subcmd__list)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__patch__subcmd__help)
            opts="create notify apply deps help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__patch__subcmd__help__subcmd__apply)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__patch__subcmd__help__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__patch__subcmd__help__subcmd__deps)
            opts="list"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__patch__subcmd__help__subcmd__deps__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__patch__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__patch__subcmd__help__subcmd__notify)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__patch__subcmd__notify)
            opts="-v -h --patch-id --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --patch-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__perf)
            opts="-v -h --method --output --flamegraph --compare --recommendations --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --method)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --flamegraph)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --compare)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help list marketplace install uninstall run config help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__config)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help get set disable enable help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__config__subcmd__disable)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__config__subcmd__enable)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__config__subcmd__get)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__config__subcmd__help)
            opts="get set disable enable help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__config__subcmd__help__subcmd__disable)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__config__subcmd__help__subcmd__enable)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__config__subcmd__help__subcmd__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__config__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__config__subcmd__help__subcmd__set)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__config__subcmd__set)
            opts="-v -h --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --json)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__help)
            opts="list marketplace install uninstall run config help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__help__subcmd__config)
            opts="get set disable enable"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__help__subcmd__config__subcmd__disable)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__help__subcmd__config__subcmd__enable)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__help__subcmd__config__subcmd__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__help__subcmd__config__subcmd__set)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 5 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__help__subcmd__install)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__help__subcmd__marketplace)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__help__subcmd__run)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__help__subcmd__uninstall)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__install)
            opts="-v -h --version --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --version)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__list)
            opts="-v -h --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__marketplace)
            opts="-v -h --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__run)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__plugins__subcmd__uninstall)
            opts="-v -h --version --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --version)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__policy)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help check help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__policy__subcmd__check)
            opts="-v -h --wasm-path --policy --explain --json --dry-run --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --wasm-path)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --policy)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__policy__subcmd__help)
            opts="check help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__policy__subcmd__help__subcmd__check)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__policy__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__profile)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help view edit update list-contracts export help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__profile__subcmd__edit)
            opts="-v -h --name --bio --website --email --github --avatar --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --name)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --bio)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --website)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --email)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --github)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --avatar)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__profile__subcmd__export)
            opts="-v -h --address --format --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --address)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__profile__subcmd__help)
            opts="view edit update list-contracts export help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__profile__subcmd__help__subcmd__edit)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__profile__subcmd__help__subcmd__export)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__profile__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__profile__subcmd__help__subcmd__list__subcmd__contracts)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__profile__subcmd__help__subcmd__update)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__profile__subcmd__help__subcmd__view)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__profile__subcmd__list__subcmd__contracts)
            opts="-v -h --address --limit --format --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --address)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --limit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__profile__subcmd__update)
            opts="-v -h --field --value --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --field)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --value)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__profile__subcmd__view)
            opts="-v -h --address --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --address)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__publish)
            opts="-v -h --contract-id --name --description --network --category --tags --publisher --contract-path --test-command --require-coverage --coverage-threshold --skip-tests --policy --explain --dry-run --api-url --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --contract-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --name)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --description)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --category)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --tags)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --publisher)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --contract-path)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --test-command)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --coverage-threshold)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --policy)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__publisher)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help doctor help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__publisher__subcmd__doctor)
            opts="-v -h --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__publisher__subcmd__help)
            opts="doctor help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__publisher__subcmd__help__subcmd__doctor)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__publisher__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__release__subcmd__notes)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help generate view edit publish list help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__release__subcmd__notes__subcmd__edit)
            opts="-v -h --contract-id --version --file --text --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --contract-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --version)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --text)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__release__subcmd__notes__subcmd__generate)
            opts="-v -h --contract-id --version --previous-version --changelog --contract-address --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --contract-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --version)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --previous-version)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --changelog)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --contract-address)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__release__subcmd__notes__subcmd__help)
            opts="generate view edit publish list help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__release__subcmd__notes__subcmd__help__subcmd__edit)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__release__subcmd__notes__subcmd__help__subcmd__generate)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__release__subcmd__notes__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__release__subcmd__notes__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__release__subcmd__notes__subcmd__help__subcmd__publish)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__release__subcmd__notes__subcmd__help__subcmd__view)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__release__subcmd__notes__subcmd__list)
            opts="-v -h --contract-id --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --contract-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__release__subcmd__notes__subcmd__publish)
            opts="-v -h --contract-id --version --skip-version-update --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --contract-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --version)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__release__subcmd__notes__subcmd__view)
            opts="-v -h --contract-id --version --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --contract-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --version)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__repl)
            opts="-v -h --network --api-url --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__scan__subcmd__deps)
            opts="-v -h --contract-id --dependencies --fail-on-high --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --contract-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --dependencies)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__search)
            opts="-v -h --verified-only --networks --category --sort --limit --offset --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --networks)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --category)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sort)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --limit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --offset)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__sign)
            opts="-v -h --private-key --contract-id --version --expires-at --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --private-key)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --contract-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --version)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --expires-at)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__sla)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help record status help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__sla__subcmd__help)
            opts="record status help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__sla__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__sla__subcmd__help__subcmd__record)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__sla__subcmd__help__subcmd__status)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__sla__subcmd__record)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__sla__subcmd__status)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__snapshot)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help export sign verify inspect help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__snapshot__subcmd__export)
            opts="-o -v -h --output --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --output)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -o)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__snapshot__subcmd__help)
            opts="export sign verify inspect help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__snapshot__subcmd__help__subcmd__export)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__snapshot__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__snapshot__subcmd__help__subcmd__inspect)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__snapshot__subcmd__help__subcmd__sign)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__snapshot__subcmd__help__subcmd__verify)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__snapshot__subcmd__inspect)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__snapshot__subcmd__sign)
            opts="-v -h --key --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --key)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__snapshot__subcmd__verify)
            opts="-v -h --trust-key --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --trust-key)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__state)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help get set dump snapshot snapshots history help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__state__subcmd__dump)
            opts="-v -h --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__state__subcmd__get)
            opts="-v -h --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__state__subcmd__help)
            opts="get set dump snapshot snapshots history help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__state__subcmd__help__subcmd__dump)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__state__subcmd__help__subcmd__get)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__state__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__state__subcmd__help__subcmd__history)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__state__subcmd__help__subcmd__set)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__state__subcmd__help__subcmd__snapshot)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__state__subcmd__help__subcmd__snapshots)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__state__subcmd__history)
            opts="-v -h --key --limit --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --key)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --limit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__state__subcmd__set)
            opts="-v -h --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__state__subcmd__snapshot)
            opts="-v -h --label --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --label)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__state__subcmd__snapshots)
            opts="-v -h --limit --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --limit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__stats)
            opts="-v -h --timeframe --format --output --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --timeframe)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --format)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__test)
            opts="-v -h --contract-path --test-command --junit --coverage --require-coverage --coverage-threshold --setup-hook --teardown-hook --mock-config --report --profile-output --load-iterations --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --contract-path)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --test-command)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --junit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --coverage-threshold)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --setup-hook)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --teardown-hook)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --mock-config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --report)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile-output)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --load-iterations)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__track__subcmd__deployment)
            opts="-v -h --contract-id --network --tx-hash --wait-timeout --json --api-url --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --contract-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --tx-hash)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --wait-timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__upgrade)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help analyze apply rollback generate help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__upgrade__subcmd__analyze)
            opts="-v -h --json --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__upgrade__subcmd__analyze)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__upgrade__subcmd__apply)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__upgrade__subcmd__generate)
            opts="-o -v -h --language --output --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --language)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -o)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__upgrade__subcmd__help)
            opts="analyze apply rollback generate help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__upgrade__subcmd__help__subcmd__analyze)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__upgrade__subcmd__help__subcmd__apply)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__upgrade__subcmd__help__subcmd__generate)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__upgrade__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__upgrade__subcmd__help__subcmd__rollback)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__upgrade__subcmd__rollback)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__verify)
            opts="-s -c -j -v -h --submit --check --history --level --json --path --notes --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --level)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --path)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --notes)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__verify__subcmd__contract)
            opts="-v -h --contract-id --version --signature --public-key --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --contract-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --version)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --signature)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --public-key)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__verify__subcmd__formal)
            opts="-v -h --properties --output --post --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --properties)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --output)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__verify__subcmd__package)
            opts="-v -h --contract-id --version --signature --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --contract-id)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --version)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --signature)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__version)
            opts="-v -h --check-updates --auto-update --rollback --api-url --network --timeout --profile --no-cache --verbose --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --rollback)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__versions)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help list bump help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__versions__subcmd__bump)
            opts="-v -h --level --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --level)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__versions__subcmd__help)
            opts="list bump help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__versions__subcmd__help__subcmd__bump)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__versions__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__versions__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__versions__subcmd__list)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__webhook)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help create list delete test logs retry verify-sig help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__webhook__subcmd__create)
            opts="-v -h --url --events --secret --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --events)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --secret)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__webhook__subcmd__delete)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__webhook__subcmd__help)
            opts="create list delete test logs retry verify-sig help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__webhook__subcmd__help__subcmd__create)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__webhook__subcmd__help__subcmd__delete)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__webhook__subcmd__help__subcmd__help)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__webhook__subcmd__help__subcmd__list)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__webhook__subcmd__help__subcmd__logs)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__webhook__subcmd__help__subcmd__retry)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__webhook__subcmd__help__subcmd__test)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__webhook__subcmd__help__subcmd__verify__subcmd__sig)
            opts=""
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 4 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__webhook__subcmd__list)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__webhook__subcmd__logs)
            opts="-v -h --limit --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --limit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__webhook__subcmd__retry)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__webhook__subcmd__test)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__webhook__subcmd__verify__subcmd__sig)
            opts="-v -h --secret --payload --signature --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 3 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --secret)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --payload)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --signature)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
        soroban__subcmd__registry__subcmd__wizard)
            opts="-v -h --api-url --network --timeout --profile --no-cache --verbose --check-updates --describe --help"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 2 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --api-url)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --network)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --profile)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
    esac
}

if [[ "${BASH_VERSINFO[0]}" -eq 4 && "${BASH_VERSINFO[1]}" -ge 4 || "${BASH_VERSINFO[0]}" -gt 4 ]]; then
    complete -F _soroban-registry -o nosort -o bashdefault -o default soroban-registry
else
    complete -F _soroban-registry -o bashdefault -o default soroban-registry
fi
