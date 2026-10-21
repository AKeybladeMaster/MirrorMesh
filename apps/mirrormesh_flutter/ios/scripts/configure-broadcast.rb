# Run on macOS with the xcodeproj gem supplied by CocoaPods before building iOS.
require 'xcodeproj'
ios_root = File.expand_path('..', __dir__)
project = Xcodeproj::Project.open(File.join(ios_root, 'Runner.xcodeproj'))
runner = project.targets.find { |target| target.name == 'Runner' } or abort 'Runner target missing'
extension = project.targets.find { |target| target.name == 'MirrorMeshBroadcast' }
unless extension
  extension = project.new_target(:app_extension, 'MirrorMeshBroadcast', :ios, '15.0')
  group = project.main_group.new_group('BroadcastExtension', 'BroadcastExtension')
  extension.source_build_phase.add_file_reference(group.new_file('SampleHandler.swift'))
  group.new_file('Info.plist')
  runner.add_dependency(extension)
  embed = runner.copy_files_build_phases.find { |phase| phase.name == 'Embed App Extensions' } || runner.new_copy_files_build_phase('Embed App Extensions')
  embed.dst_subfolder_spec = '13'
  embed.add_file_reference(extension.product_reference).settings = {'ATTRIBUTES' => ['RemoveHeadersOnCopy']}
end
runner.build_configurations.each do |configuration|
  settings = configuration.build_settings
  settings['MIRRORMESH_APP_GROUP'] ||= ENV.fetch('MM_IOS_APP_GROUP', 'group.com.mirrormesh.screen')
  settings['MIRRORMESH_BROADCAST_ID'] = ENV.fetch('MM_IOS_BROADCAST_ID', 'com.mirrormesh.app.broadcast')
  settings['CODE_SIGN_ENTITLEMENTS'] = 'Runner/Broadcast.entitlements'
  settings['IPHONEOS_DEPLOYMENT_TARGET'] = '15.0'
end
extension.build_configurations.each do |configuration|
  settings = configuration.build_settings
  runner_settings = runner.build_configurations.find { |item| item.name == configuration.name }&.build_settings || runner.build_configurations.first.build_settings
  settings['PRODUCT_BUNDLE_IDENTIFIER'] = runner_settings['MIRRORMESH_BROADCAST_ID']
  settings['INFOPLIST_FILE'] = 'BroadcastExtension/Info.plist'
  settings['CODE_SIGN_ENTITLEMENTS'] = 'Runner/Broadcast.entitlements'
  settings['MIRRORMESH_APP_GROUP'] = runner_settings['MIRRORMESH_APP_GROUP']
  settings['SWIFT_VERSION'] = '5.0'
  settings['SKIP_INSTALL'] = 'YES'
  settings['APPLICATION_EXTENSION_API_ONLY'] = 'YES'
  settings['DEVELOPMENT_TEAM'] = runner_settings['DEVELOPMENT_TEAM'] if runner_settings['DEVELOPMENT_TEAM']
end
project.save
puts 'Broadcast configured. Set your Apple team, unique bundle IDs and provisioned app group in Xcode.'
